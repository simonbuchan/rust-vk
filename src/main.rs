use ash::prelude::VkResult;
use ash::vk;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use device::{AsRawHandle, CommandBuffer, CommandBufferRenderPassRecorder};
use renderer::{PipelineBuilder, Renderer};

mod device;
mod globals;
mod init;
mod renderer;

fn main() {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let surface = init::init(&window).unwrap();
    let mut mesh_renderer = MeshRenderer::new(window, surface).unwrap();

    event_loop.run(move |event, _loop, flow| match event {
        Event::NewEvents(..) => {
            *flow = ControlFlow::Wait;
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => {
                *flow = ControlFlow::Exit;
            }
            WindowEvent::Resized(size) => {
                mesh_renderer.resize(size.into()).unwrap();
            }
            _ => {}
        },
        Event::RedrawRequested(..) => {
            mesh_renderer.render().unwrap();
        }
        _ => {}
    });
}

struct Mesh {
    pub memory: device::Memory,
    pub vertex_buffer: device::BufferObject,
    pub index_buffer: device::BufferObject,
    pub count: u32,
}

impl Mesh {
    pub fn create<Vertex: Copy>(vertices: &[Vertex], indices: &[u32]) -> VkResult<Self> {
        let vertex_buffer = device::BufferObject::create(
            device::size_of_val(vertices),
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer = device::BufferObject::create(
            device::size_of_val(indices),
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;

        // compute combined memory requirements
        let vertex_requirements = vertex_buffer.memory_requirements();
        let index_requirements = index_buffer.memory_requirements();
        let merged_size = vertex_requirements.size + index_requirements.size;

        let vertices_offset = 0;
        let indices_offset = vertex_requirements.size;

        let memory = device::Memory::allocate_mappable(
            merged_size,
            device::MemoryTypeMask(
                vertex_requirements.memory_type_bits & index_requirements.memory_type_bits,
            ),
        )?;

        // write memory
        let mapping = memory.map(0, merged_size)?;
        unsafe {
            mapping.write(vertices_offset, vertices);
            mapping.write(indices_offset, indices);
        }

        // bind buffers to memory
        vertex_buffer.bind_memory(&memory, vertices_offset)?;
        index_buffer.bind_memory(&memory, indices_offset)?;

        Ok(Self {
            memory,
            vertex_buffer,
            index_buffer,
            count: indices.len() as u32,
        })
    }

    pub fn draw(&self, cmd: &CommandBufferRenderPassRecorder) {
        cmd.bind_vertex_buffer(0, self.vertex_buffer.as_raw());
        cmd.bind_index_buffer(self.index_buffer.as_raw());
        cmd.draw_indexed(self.count);
    }
}

pub struct Texture {
    pub image: device::Image,
    pub image_view: device::ImageView,
    pub sampler: device::Sampler,
}

#[allow(dead_code)]
struct MeshRenderer {
    window: Window,
    renderer: Renderer,
    surface: device::Owned<vk::SurfaceKHR>,
    pipeline_builder: PipelineBuilder,
    pipeline: device::Owned<vk::Pipeline>,
    pipeline_layout: device::Owned<vk::PipelineLayout>,
    descriptor_pool: device::DescriptorPool,
    descriptor_set: device::DescriptorSet,
    mvp_buffer: device::Buffer,
    mesh: Mesh,
    texture: Texture,
}

impl MeshRenderer {
    pub fn new(window: Window, surface: device::Owned<vk::SurfaceKHR>) -> VkResult<Self> {
        let renderer = Renderer::create(surface.as_raw(), window.inner_size().into())?;

        let pipeline_builder = renderer
            .pipeline_builder()?
            .add_vertex_input_vec3(0)
            .add_vertex_input_vec4(1)
            .add_vertex(
                r#"
                    #version 460
                    layout(set = 0, binding = 0) uniform per_draw { mat4 u_mvp; };
                    layout(location = 0) in vec3 a_pos;
                    layout(location = 1) in vec4 a_col;
                    layout(location = 0) out vec2 v_uv;
                    layout(location = 1) out vec4 v_col;
                    void main() {
                        v_uv = a_pos.xy;
                        gl_Position = u_mvp * vec4(a_pos, 1);
                        v_col = a_col;
                    }
                "#,
            )
            .add_fragment(
                r#"
                    #version 460
                    layout(set = 0, binding = 1) uniform sampler2D u_tex;
                    layout(location = 0) in vec2 v_uv;
                    layout(location = 1) in vec4 v_col;
                    layout(location = 0) out vec4 o_col;
                    void main() {
                        o_col = texture(u_tex, v_uv) * v_col;
                    }
                "#,
            );

        let mvp_buffer = device::Buffer::create_with::<[[f32; 4]; 4]>(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            &[[
                [2., 0., 0., 0.],
                [0., 2., 0., 0.],
                [0., 0., 1., 0.],
                [-1., -1., 0., 1.],
            ]],
        )?;

        const IMAGE_SIZE: (u32, u32) = (128, 128);
        let image = device::Image::create_2d(
            IMAGE_SIZE,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::SAMPLED,
            vk::ImageLayout::PREINITIALIZED,
            device::MemoryTypeMask::mappable(),
        )?;

        {
            let layout = image.object.color_subresource_layout();
            let map = image.memory.map(layout.offset, layout.size)?;
            for i in 0..IMAGE_SIZE.1 {
                let v = ((if i / 8 % 2 == 0 {
                    i
                } else {
                    IMAGE_SIZE.1 - i - 1
                }) * 256
                    / IMAGE_SIZE.1) as u8;
                unsafe {
                    map.write::<[u8; 4]>(
                        layout.row_pitch * i as vk::DeviceSize,
                        &[[v, v, v, 255u8]; IMAGE_SIZE.0 as usize],
                    )
                };
            }
        }

        let image_view = device::ImageView::create_2d(
            image.object.as_raw(),
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageAspectFlags::COLOR,
        )?;

        let sampler = device::Sampler::nearest()?;

        let set_layout = device::DescriptorSetLayout::builder()
            .add_uniform_buffer(0, vk::ShaderStageFlags::VERTEX)
            .add_combined_image_sampler(1, vk::ShaderStageFlags::FRAGMENT)
            .build()?;

        let pipeline_layout = unsafe {
            device::Owned::<vk::PipelineLayout>::create(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[set_layout.as_raw()])
                    .build(),
            )?
        };

        let descriptor_pool = device::DescriptorPool::create(
            1,
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

        let descriptor_set = descriptor_pool.allocate(set_layout.as_raw())?;

        descriptor_set.update_buffer(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            mvp_buffer.as_raw(),
            0,
            64,
        );
        descriptor_set.update_combined_image_sampler(
            1,
            sampler.as_raw(),
            image_view.as_raw(),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        drop(set_layout);

        let recorder = device::CommandBuffer::create()?;
        recorder.image_memory_barrier(
            vk::PipelineStageFlags::HOST,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            &vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::HOST_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::PREINITIALIZED)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(image.object.as_raw())
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .level_count(1)
                        .layer_count(1)
                        .build(),
                ),
        );
        recorder.end()?.submit()?;

        let pipeline = pipeline_builder.build(renderer.size, pipeline_layout.as_raw())?;

        let mesh = Mesh::create::<[f32; 7]>(
            &[
                [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0],
                [0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0],
                [1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            ],
            &[0, 1, 2, 2, 1, 3],
        )?;

        let texture = Texture {
            image,
            image_view,
            sampler,
        };

        Ok(Self {
            window,
            surface,
            renderer,
            pipeline_builder,
            pipeline_layout,
            pipeline,
            descriptor_pool,
            descriptor_set,
            mvp_buffer,
            mesh,
            texture,
        })
    }

    pub fn resize(&mut self, size: (u32, u32)) -> VkResult<()> {
        self.renderer.resize(size.into())?;
        self.pipeline = self
            .pipeline_builder
            .build(size.into(), self.pipeline_layout.as_raw())?;
        Ok(())
    }

    pub fn render(&mut self) -> VkResult<()> {
        use globals::*;
        let swapchain_item = self.renderer.acquire_image()?;
        let (width, height) = self.renderer.size;

        let recorder = CommandBuffer::create()?;

        let recorder = recorder.begin_render_pass(
            &vk::RenderPassBeginInfo::builder()
                .render_pass(self.renderer.render_pass.as_raw())
                .framebuffer(swapchain_item.framebuffer)
                .clear_values(&[vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.2, 0.5, 0.7, 1.0],
                    },
                }])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D { width, height },
                }),
        );

        recorder.bind_pipeline(self.pipeline.as_raw());
        recorder.bind_descriptor_set(self.pipeline_layout.as_raw(), self.descriptor_set.as_raw());
        self.mesh.draw(&recorder);
        let command_buffer = recorder.end_render_pass().end()?;
        command_buffer.submit_after(
            self.renderer.image_acquire_semaphore.as_raw(),
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        )?;

        self.renderer.present(swapchain_item.index)?;

        Ok(())
    }
}
