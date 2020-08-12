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

struct Material {
    descriptors_layout: device::DescriptorSetLayout,
    pipeline_layout: device::PipelineLayout,
    pipeline: device::Pipeline,
}

impl Material {
    fn create(
        definition: &definition::Material,
        compiler: &mut resources::Compiler,
        view_descriptors_layout: vk::DescriptorSetLayout,
        render_pass: vk::RenderPass,
        samples: vk::SampleCountFlags,
    ) -> Result<Self> {
        let mut layout_builder = device::DescriptorSetLayout::builder();
        for descriptor in &definition.descriptors {
            layout_builder = layout_builder.add_basic(
                descriptor.binding,
                descriptor.ty.into(),
                descriptor
                    .stages
                    .iter()
                    .fold(vk::ShaderStageFlags::empty(), |a, &s| a | s.into()),
            );
        }
        let descriptors_layout = layout_builder.build()?;

        let pipeline_layout = device::PipelineLayout::create(
            &[view_descriptors_layout, descriptors_layout.as_raw()],
            &[vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::VERTEX,
                offset: 0,
                size: std::mem::size_of::<Mat4>() as u32,
            }],
        )?;

        let vs = compiler.compile_vertex(&definition.vertex)?;
        let fs = compiler.compile_fragment(&definition.fragment)?;

        let pipeline = create_pipeline(
            None,
            render_pass,
            &[vs, fs],
            &definition.vertex_input,
            pipeline_layout.as_raw(),
            samples,
        )?;

        Ok(Self {
            descriptors_layout,
            pipeline_layout,
            pipeline,
        })
    }
}

#[allow(dead_code)]
pub struct Scene {
    material: Material,
    descriptor_pool: device::DescriptorPool,
    view_set: device::DescriptorSet,
    material_set: device::DescriptorSet,
    view_uniform_buffer: device::Buffer,
    camera: camera::Camera<camera::PerspectiveProjection>,
    mesh_transform: Mat4,
    mesh: resources::Mesh,
    texture: resources::Texture,
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

        let material = Material::create(
            &scene.materials[0],
            &mut compiler,
            view_descriptors_layout.as_raw(),
            render_pass,
            samples,
        )?;

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

        let view_set = descriptor_pool.allocate(view_descriptors_layout.as_raw())?;
        let material_set = descriptor_pool.allocate(material.descriptors_layout.as_raw())?;

        let view_uniform_buffer = device::Buffer::create(
            device::size_of::<ViewUniforms>(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;

        let (info, mut reader) =
            png::Decoder::new(std::fs::File::open("assets/TextureCoordinateTemplate.png")?)
                .read_info()?;

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

        view_set.update_buffer(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            view_uniform_buffer.as_raw(),
            0,
            device::size_of::<ViewUniforms>(),
        );
        material_set.update_combined_image_sampler(
            0,
            texture.sampler.as_raw(),
            texture.image_view.as_raw(),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        let camera = camera::Camera {
            transform: camera::Transform {
                position: [0.0, 0.0, -5.0].into(),
                rotation: Quaternion::ZERO,
            },
            projection: camera::PerspectiveProjection {
                aspect: 4.0 / 3.0,
                fov_height: 60.0 * std::f32::consts::PI / 180.0,
                near: 0.1,
                far: 100.0,
            },
            // projection: camera::OrthographicProjection {
            //     width: 4.0,
            //     height: 3.0,
            //     depth: 20.0,
            // },
        };

        let mesh = create_box_mesh()?;

        Ok(Self {
            material,
            descriptor_pool,
            view_set,
            material_set,
            view_uniform_buffer,
            camera,
            mesh_transform: Mat4::IDENTITY,
            mesh,
            texture,
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
        self.camera.transform.position = rotate_around.rotate([0.0, 3.0, 5.0].into());
    }

    pub fn render(&self, recorder: &device::CommandBufferRenderPassRecorder) -> Result<()> {
        recorder.bind_pipeline(self.material.pipeline.as_raw());

        self.view_uniform_buffer.write(
            0,
            &ViewUniforms {
                view: self.camera.transform.matrix(),
                proj: self.camera.projection.matrix(),
            },
        )?;
        recorder.bind_descriptor_set(
            self.material.pipeline_layout.as_raw(),
            0,
            self.view_set.as_raw(),
        );
        recorder.bind_descriptor_set(
            self.material.pipeline_layout.as_raw(),
            1,
            self.material_set.as_raw(),
        );
        recorder.push(
            self.material.pipeline_layout.as_raw(),
            vk::ShaderStageFlags::VERTEX,
            0,
            &self.mesh_transform,
        );
        self.mesh.draw(&recorder);
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

fn create_pipeline(
    cache: Option<vk::PipelineCache>,
    render_pass: vk::RenderPass,
    stages: &[resources::Shader],
    bindings: &[definition::VertexInputBinding],
    layout: vk::PipelineLayout,
    samples: vk::SampleCountFlags,
) -> VkResult<device::Pipeline> {
    let name = std::ffi::CString::new("main").unwrap();
    device::Pipeline::create(
        cache,
        &vk::GraphicsPipelineCreateInfo::builder()
            .stages(
                &stages
                    .iter()
                    .map(|stage| {
                        vk::PipelineShaderStageCreateInfo::builder()
                            .name(&name)
                            .module(stage.as_raw())
                            .stage(stage.stage())
                            .build()
                    })
                    .collect::<Vec<_>>(),
            )
            .vertex_input_state(
                &vk::PipelineVertexInputStateCreateInfo::builder()
                    .vertex_binding_descriptions(
                        &bindings
                            .iter()
                            .map(|b| {
                                vk::VertexInputBindingDescription::builder()
                                    .binding(b.binding)
                                    .stride(b.stride)
                                    .input_rate(vk::VertexInputRate::VERTEX)
                                    .build()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .vertex_attribute_descriptions({
                        &bindings
                            .iter()
                            .flat_map(|b| {
                                b.attributes
                                    .iter()
                                    .map(|a| {
                                        vk::VertexInputAttributeDescription::builder()
                                            .binding(b.binding)
                                            .location(a.location)
                                            .offset(a.offset)
                                            .format(a.format.into())
                                            .build()
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>()
                    }),
            )
            .input_assembly_state(
                &vk::PipelineInputAssemblyStateCreateInfo::builder()
                    .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                    .build(),
            )
            .viewport_state(
                &vk::PipelineViewportStateCreateInfo::builder()
                    .viewport_count(1)
                    .scissor_count(1)
                    .build(),
            )
            .rasterization_state(
                &vk::PipelineRasterizationStateCreateInfo::builder()
                    .line_width(1.0)
                    .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                    .cull_mode(vk::CullModeFlags::BACK)
                    .build(),
            )
            .multisample_state(
                &vk::PipelineMultisampleStateCreateInfo::builder()
                    .rasterization_samples(samples)
                    .build(),
            )
            .color_blend_state(
                &vk::PipelineColorBlendStateCreateInfo::builder()
                    .attachments(&[vk::PipelineColorBlendAttachmentState::builder()
                        .color_write_mask(vk::ColorComponentFlags::all())
                        .blend_enable(true)
                        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                        .src_color_blend_factor(vk::BlendFactor::SRC_COLOR)
                        .color_blend_op(vk::BlendOp::ADD)
                        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                        .src_alpha_blend_factor(vk::BlendFactor::ONE)
                        .alpha_blend_op(vk::BlendOp::ADD)
                        .build()])
                    .build(),
            )
            .dynamic_state(
                &vk::PipelineDynamicStateCreateInfo::builder()
                    .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]),
            )
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0)
            .build(),
    )
}
