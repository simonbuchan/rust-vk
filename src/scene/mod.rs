use std::collections::BTreeMap;
use std::time::Duration;

use ash::prelude::VkResult;
use ash::vk;

use crate::device::{self, AsRawHandle};
use crate::error::*;
use crate::math::*;
use crate::resources;
use crate::scene::camera::Projection;
use notify::Watcher;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

mod camera;
mod definition;
mod material;
mod textures;

struct Model {
    transform: Mat4,
    material: u32,
    mesh: resources::MeshObject,
}

#[derive(Copy, Clone)]
struct ViewUniforms {
    pub view: Mat4,
    pub proj: Mat4,
    pub camera_position: Vec3,
}

pub struct SceneWatcher {
    change_watcher: notify::RecommendedWatcher,
    change_receiver: std::sync::mpsc::Receiver<notify::DebouncedEvent>,
    render_pass: vk::RenderPass,
    samples: vk::SampleCountFlags,
    path: PathBuf,
    scene: Scene,
    watch_paths: Vec<PathBuf>,
}

impl SceneWatcher {
    pub fn create(
        render_pass: vk::RenderPass,
        samples: vk::SampleCountFlags,
        path: &Path,
    ) -> Result<Self> {
        let (tx, change_receiver) = std::sync::mpsc::channel();
        let mut change_watcher =
            notify::RecommendedWatcher::new(tx, std::time::Duration::from_millis(100)).unwrap();
        change_watcher
            .watch(path, notify::RecursiveMode::NonRecursive)
            .unwrap();

        let (scene, watch_paths) = Scene::parse(render_pass, samples, &path)?;

        for p in &watch_paths {
            change_watcher
                .watch(&p, notify::RecursiveMode::NonRecursive)
                .unwrap();
        }

        Ok(Self {
            change_watcher,
            change_receiver,
            render_pass,
            samples,
            path: PathBuf::from(path),
            scene,
            watch_paths,
        })
    }

    fn check_reload(&mut self) -> Result<()> {
        if let Ok(change) = self.change_receiver.try_recv() {
            if matches!(
                change,
                notify::DebouncedEvent::NoticeWrite(..) | notify::DebouncedEvent::NoticeRemove(..)
            ) {
                return Ok(());
            }
            println!("change: {:?}", change);

            while self.change_receiver.try_recv().is_ok() {}

            match Scene::parse(self.render_pass, self.samples, &self.path) {
                Err(err) => {
                    eprintln!("failed to parse: {:?}", err);
                }
                Ok((mut scene, watch_paths)) => {
                    for p in &self.watch_paths {
                        self.change_watcher.unwatch(&p).unwrap();
                    }
                    for p in &watch_paths {
                        self.change_watcher
                            .watch(&p, notify::RecursiveMode::NonRecursive)
                            .unwrap();
                    }

                    // Preserve aspect ratio (should probably be recomputed each frame?)
                    scene.camera.projection = self.scene.camera.projection;

                    self.scene = scene;
                    self.watch_paths = watch_paths;
                }
            }
        }

        Ok(())
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        self.scene.resize(size);
    }

    pub fn update(&mut self, elapsed: Duration) {
        self.check_reload().unwrap();
        self.scene.update(elapsed);
    }

    pub fn render(&self, recorder: &device::CommandBufferRenderPassRecorder) -> Result<()> {
        self.scene.render(recorder)
    }
}

#[allow(dead_code)]
pub struct Scene {
    programs: BTreeMap<u32, material::MaterialProgram>,
    materials: BTreeMap<u32, material::Material>,
    textures: BTreeMap<u32, resources::Texture>,
    memories: Vec<device::Memory>,
    models: Vec<Model>,
    descriptor_pool: device::DescriptorPool,
    view_set: device::DescriptorSet,
    view_uniform_buffer: device::Buffer,
    camera: camera::Camera<camera::PerspectiveProjection>,
}

impl Scene {
    pub fn parse(
        render_pass: vk::RenderPass,
        samples: vk::SampleCountFlags,
        path: &Path,
    ) -> Result<(Self, Vec<PathBuf>)> {
        let scene: definition::Scene = serde_yaml::from_reader(std::fs::File::open(path)?)?;

        let view_descriptors_layout = device::DescriptorSetLayout::builder()
            .add_uniform_buffer(0, vk::ShaderStageFlags::ALL)
            .build()?;

        let mut compiler = resources::Compiler::new();

        let mut programs = BTreeMap::new();
        for p in &scene.programs {
            programs.insert(
                p.id,
                material::MaterialProgram::create(
                    p,
                    &mut compiler,
                    view_descriptors_layout.as_raw(),
                )?,
            );
        }

        let descriptor_pool = device::DescriptorPool::create(
            1 + scene.textures.len() as u32,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: scene.textures.len() as u32,
                },
            ],
        )?;

        let mut paths = vec![];

        let mut textures = BTreeMap::new();
        for t in &scene.textures {
            paths.push(PathBuf::from(&t.path));

            textures.insert(
                t.id,
                match t.format {
                    definition::TextureFormat::Ktx => textures::load_ktx(&t.path),
                    definition::TextureFormat::Png => textures::load_png(
                        &t.path,
                        matches!(t.space, definition::TextureColorSpace::Srgb),
                    ),
                }?,
            );
        }

        let view_set = descriptor_pool.allocate(view_descriptors_layout.as_raw())?;

        let view_uniform_buffer = device::Buffer::create(
            device::size_of::<ViewUniforms>(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;

        let mut materials = BTreeMap::new();
        for m in &scene.materials {
            let program = &programs[&m.program];
            let pipeline = program.create_material_pipeline(render_pass, samples)?;
            let descriptors = descriptor_pool.allocate(program.descriptors_layout.as_raw())?;
            for t in &m.textures {
                let texture = &textures[&t.texture];
                descriptors.update_combined_image_sampler(
                    t.location,
                    texture.sampler.as_raw(),
                    texture.image_view.as_raw(),
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                );
            }
            materials.insert(
                m.id,
                material::Material {
                    program: m.program,
                    pipeline,
                    descriptors,
                },
            );
        }

        let mut buffer_files = BTreeMap::new();
        let mut memories = Vec::new();

        for b in &scene.buffers {
            buffer_files.insert(b.id, std::fs::File::open(&b.path)?);
            paths.push(PathBuf::from(&b.path));
        }

        let mut buffer_view = |view: &definition::BufferView,
                               usage: vk::BufferUsageFlags|
         -> Result<device::BufferObject> {
            let mut file = &buffer_files[&view.buffer];
            let buffer = device::Buffer::create(view.size, usage)?;
            let mut mapping = buffer.memory.map(0, view.size as usize)?;
            file.seek(std::io::SeekFrom::Start(view.offset))?;
            file.read_exact(mapping.slice(0, view.size as usize))?;
            drop(mapping);
            memories.push(buffer.memory);
            Ok(buffer.object)
        };

        let mut models = Vec::new();
        for m in &scene.meshes {
            let mut vertex_buffers = Vec::new();
            for b in &m.bindings {
                vertex_buffers.push(buffer_view(&b.view, vk::BufferUsageFlags::VERTEX_BUFFER)?);
            }
            let index_buffer = buffer_view(&m.indices.view, vk::BufferUsageFlags::INDEX_BUFFER)?;
            models.push(Model {
                transform: (&m.transform).into(),
                material: m.material,
                mesh: resources::MeshObject {
                    vertex_buffers,
                    index_buffer,
                    index_type: m.indices.format.into(),
                    index_count: m.indices.count,
                },
            });
        }

        view_set.update_buffer(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            view_uniform_buffer.as_raw(),
            0,
            device::size_of::<ViewUniforms>(),
        );

        let camera = camera::Camera {
            transform: Default::default(),
            projection: camera::PerspectiveProjection {
                near: 0.00001,
                far: 100.0,
                ..Default::default()
            },
        };

        let scene = Self {
            programs,
            materials,
            textures,
            memories,
            models,
            descriptor_pool,
            view_set,
            view_uniform_buffer,
            camera,
        };

        Ok((scene, paths))
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        let aspect = size.0 as f32 / size.1 as f32;
        self.camera.projection.aspect = aspect;
    }

    pub fn update(&mut self, elapsed: Duration) {
        let rotate_around = Quaternion::axis_angle(Vec3::Y_POS, elapsed.as_secs_f32() * 0.3);
        let rotate_down = Quaternion::axis_angle(Vec3::X_NEG, std::f32::consts::FRAC_PI_6);
        self.camera.transform.rotation = rotate_around * rotate_down;
        self.camera.transform.position = rotate_around.rotate([0.0, 0.02, 0.04].into());
    }

    pub fn render(&self, recorder: &device::CommandBufferRenderPassRecorder) -> Result<()> {
        self.view_uniform_buffer.write(
            0,
            &ViewUniforms {
                view: self.camera.transform.matrix(),
                proj: self.camera.projection.matrix(),
                camera_position: self.camera.transform.position,
            },
        )?;

        // TODO: sort by program (pipeline_layout) / material (pipeline)
        for model in &self.models {
            let material = &self.materials[&model.material];
            let program = &self.programs[&material.program];
            recorder.bind_pipeline(material.pipeline.as_raw());
            let pipeline_layout = program.pipeline_layout.as_raw();
            recorder.bind_descriptor_set(pipeline_layout, 0, self.view_set.as_raw());

            // for each material
            recorder.bind_descriptor_set(pipeline_layout, 1, material.descriptors.as_raw());

            // for each model
            recorder.push(
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                &model.transform,
            );
            model.mesh.draw(&recorder);
        }

        Ok(())
    }
}

#[allow(dead_code)]
struct MeshBuilder<V: Copy> {
    vertices: Vec<V>,
    indices: Vec<u32>,
}

#[allow(dead_code)]
impl<V: Copy> MeshBuilder<V> {
    pub fn new() -> Self {
        Self {
            vertices: vec![],
            indices: vec![],
        }
    }

    pub fn quad(&mut self, tl: V, tr: V, bl: V, br: V) {
        let index = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&[tl, tr, bl, br]);
        self.indices
            .extend_from_slice(&[index, index + 2, index + 1]); // TL, BL, TR
        self.indices
            .extend_from_slice(&[index + 1, index + 2, index + 3]); // TR, BL, BR
    }

    pub fn build(&self) -> VkResult<resources::Mesh> {
        resources::Mesh::create::<V>(&self.vertices, &self.indices)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Vertex(pub Vec3, pub Vec2, pub Vec4);

fn create_box_mesh() -> VkResult<resources::Mesh> {
    // "out" (determining facing) and "up" (determining texture orientation).
    const FACES: &[(Vec3, Vec3)] = &[
        (Vec3::X_POS, Vec3::Y_POS),
        (Vec3::X_NEG, Vec3::Y_POS),
        (Vec3::Y_POS, Vec3::Z_POS),
        (Vec3::Y_NEG, Vec3::Z_POS),
        (Vec3::Z_POS, Vec3::Y_POS),
        (Vec3::Z_NEG, Vec3::Y_POS),
    ];

    let mut builder = MeshBuilder::new();
    for &(out, up) in FACES {
        let right = Vec3::cross(up, out);
        let bl = out + -up + -right;
        let br = out + -up + right;
        let tl = out + up + -right;
        let tr = out + up + right;

        fn vertex(pos: Vec3, u: f32, v: f32) -> Vertex {
            Vertex(pos, [u, v].into(), Vec4::from((pos + Vec3::ONE) * 0.5))
        }

        builder.quad(
            vertex(tl, 0.0, 0.0),
            vertex(tr, 1.0, 0.0),
            vertex(bl, 0.0, 1.0),
            vertex(br, 1.0, 1.0),
        );
    }

    builder.build()
}
