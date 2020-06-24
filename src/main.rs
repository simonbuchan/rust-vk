use ash::prelude::VkResult;
use ash::vk;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use device::{AsRawHandle, CommandBuffer};
use renderer::{PipelineBuilder, Renderer};

mod device;
mod globals;
mod init;
mod renderer;
mod resources;

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

#[allow(dead_code)]
struct MeshRenderer {
    window: Window,
    renderer: Renderer,
    surface: device::Owned<vk::SurfaceKHR>,
    pipeline_builder: PipelineBuilder,
    pipeline: device::Pipeline,
    pipeline_layout: device::PipelineLayout,
    descriptor_pool: device::DescriptorPool,
    mvp_set: device::DescriptorSet,
    mesh_set: device::DescriptorSet,
    mvp_buffer: device::Buffer,
    mesh: resources::Mesh,
    texture: resources::Texture,
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
                    layout(set = 1, binding = 0) uniform sampler2D u_tex;
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
            &[
                [2., 0., 0., 0.],
                [0., 2., 0., 0.],
                [0., 0., 1., 0.],
                [-1., -1., 0., 1.],
            ],
        )?;

        let texture = resources::Texture::create_2d(128, 128)?;

        let mut upload = texture.begin_upload().unwrap();
        for y in 0..texture.height {
            let row = upload.row(y);
            for x in 0..texture.width {
                let is_stripe = y / 8 % 2 == 0;
                let stripe_y = if is_stripe { y } else { texture.height - y - 1 };
                let a = (x * 256 / texture.width) as u8;
                let v = (stripe_y * 256 / texture.height) as u8;
                row[x as usize] = [v, v, v, a];
            }
        }
        upload
            .upload_before(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .unwrap();

        let mvp_set_layout = device::DescriptorSetLayout::builder()
            .add_uniform_buffer(0, vk::ShaderStageFlags::VERTEX)
            .build()?;
        let mesh_set_layout = device::DescriptorSetLayout::builder()
            .add_combined_image_sampler(0, vk::ShaderStageFlags::FRAGMENT)
            .build()?;

        let pipeline_layout =
            device::PipelineLayout::create(&[mvp_set_layout.as_raw(), mesh_set_layout.as_raw()])?;

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

        let mvp_set = descriptor_pool.allocate(mvp_set_layout.as_raw())?;
        let mesh_set = descriptor_pool.allocate(mesh_set_layout.as_raw())?;

        mvp_set.update_buffer(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            mvp_buffer.as_raw(),
            0,
            64,
        );
        mesh_set.update_combined_image_sampler(
            0,
            texture.sampler.as_raw(),
            texture.image_view.as_raw(),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        let pipeline = pipeline_builder.build(renderer.size, pipeline_layout.as_raw())?;

        let mesh = resources::Mesh::create::<[f32; 7]>(
            &[
                [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0],
                [0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0],
                [1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            ],
            &[0, 1, 2, 2, 1, 3],
        )?;

        Ok(Self {
            window,
            surface,
            renderer,
            pipeline_builder,
            pipeline_layout,
            pipeline,
            descriptor_pool,
            mvp_set,
            mesh_set,
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

        recorder.bind_descriptor_set(self.pipeline_layout.as_raw(), 0, self.mvp_set.as_raw());
        recorder.bind_descriptor_set(self.pipeline_layout.as_raw(), 1, self.mesh_set.as_raw());
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
