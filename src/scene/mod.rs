use std::collections::BTreeMap;
use std::time::Duration;

use ash::prelude::VkResult;
use ash::vk;

use crate::device::{self, AsRawHandle};
use crate::error::*;
use crate::math::*;
use crate::resources;
use crate::scene::camera::Projection;

mod camera;
mod definition;
mod material;

#[allow(dead_code)]
pub struct Scene {
    programs: BTreeMap<u32, material::MaterialProgram>,
    materials: BTreeMap<u32, material::Material>,
    textures: BTreeMap<u32, resources::Texture>,
    descriptor_pool: device::DescriptorPool,
    view_set: device::DescriptorSet,
    view_uniform_buffer: device::Buffer,
    camera: camera::Camera<camera::PerspectiveProjection>,
    model: Model,
}

struct Model {
    transform: Mat4,
    mesh: resources::Mesh,
    material: u32,
}

#[derive(Copy, Clone)]
struct ViewUniforms {
    pub view: Mat4,
    pub proj: Mat4,
}

impl Scene {
    pub fn parse(
        render_pass: vk::RenderPass,
        samples: vk::SampleCountFlags,
        path: &str,
    ) -> Result<Self> {
        let scene: definition::Scene = serde_yaml::from_reader(std::fs::File::open(path)?)?;

        let view_descriptors_layout = device::DescriptorSetLayout::builder()
            .add_uniform_buffer(0, vk::ShaderStageFlags::ALL)
            .build()?;

        let mut compiler = resources::Compiler::new();

        let mut programs = BTreeMap::new();
        let mut materials = BTreeMap::new();
        let mut textures = BTreeMap::new();

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
            2,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                },
            ],
        )?;

        for t in &scene.textures {
            let (info, mut reader) =
                png::Decoder::new(std::fs::File::open(&t.path)?).read_info()?;

            let texture_buffer_size = reader.output_buffer_size();
            let texture_buffer = device::Buffer::create(
                texture_buffer_size as vk::DeviceSize,
                vk::BufferUsageFlags::TRANSFER_SRC,
            )?;
            let mut mapping = texture_buffer.memory.map(0, texture_buffer_size)?;
            reader.next_frame(mapping.slice(0, texture_buffer_size))?;
            drop(mapping);

            let texture = resources::Texture::create(info.width, info.height)?;
            texture.copy_from(texture_buffer.as_raw(), 0)?;
            textures.insert(t.id, texture);
        }

        let view_set = descriptor_pool.allocate(view_descriptors_layout.as_raw())?;

        let view_uniform_buffer = device::Buffer::create(
            device::size_of::<ViewUniforms>(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;

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

        view_set.update_buffer(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            view_uniform_buffer.as_raw(),
            0,
            device::size_of::<ViewUniforms>(),
        );

        let camera = camera::Camera::<camera::PerspectiveProjection>::default();

        let mesh = create_box_mesh()?;

        Ok(Self {
            programs,
            materials,
            textures,
            descriptor_pool,
            view_set,
            view_uniform_buffer,
            camera,
            model: Model {
                transform: Mat4::IDENTITY,
                mesh,
                material: 1,
            },
        })
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        let aspect = size.0 as f32 / size.1 as f32;
        self.camera.projection.aspect = aspect;
    }

    pub fn update(&mut self, elapsed: Duration) {
        let rotate_around = Quaternion::axis_angle(Vec3::Y_POS, elapsed.as_secs_f32() * 0.3);
        let rotate_down = Quaternion::axis_angle(Vec3::X_NEG, std::f32::consts::FRAC_PI_6);
        self.camera.transform.rotation = rotate_around * rotate_down;
        self.camera.transform.position = rotate_around.rotate([0.0, 1.5, 3.0].into());
    }

    pub fn render(&self, recorder: &device::CommandBufferRenderPassRecorder) -> Result<()> {
        self.view_uniform_buffer.write(
            0,
            &ViewUniforms {
                view: self.camera.transform.matrix(),
                proj: self.camera.projection.matrix(),
            },
        )?;

        let material = &self.materials[&self.model.material];
        let program = &self.programs[&material.program];
        recorder.bind_pipeline(material.pipeline.as_raw());
        let pipeline_layout = program.pipeline_layout.as_raw();
        recorder.bind_descriptor_set(pipeline_layout, 0, self.view_set.as_raw());

        // for each material
        recorder.bind_descriptor_set(pipeline_layout, 1, material.descriptors.as_raw());

        // for each model
        recorder.push(
            pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            &self.model.transform,
        );
        self.model.mesh.draw(&recorder);

        Ok(())
    }
}

struct MeshBuilder<V: Copy> {
    vertices: Vec<V>,
    indices: Vec<u32>,
}

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
