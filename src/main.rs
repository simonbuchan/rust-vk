use std::time::{Duration, Instant};

use ash::prelude::VkResult;
use ash::vk;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use device::{AsRawHandle, CommandBuffer};
use error::*;
use math::*;
use renderer::{PipelineBuilder, Renderer, VertexLayout};

mod device;
mod ecs;
mod error;
mod globals;
mod init;
mod math;
mod render_graph;
mod renderer;
mod resources;

fn main() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("{}", info);
        std::process::exit(3);
    }));

    if let Err(err) = run() {
        eprintln!("Failed: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).map_err(Error::Window)?;
    let surface = init::init(&window)?;
    let mut render_context = RenderContext::new(window.inner_size().into(), surface)?;

    std::mem::forget(window);

    event_loop.run(move |event, _loop, flow| match event {
        Event::NewEvents(..) => {
            *flow = ControlFlow::Poll;
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => {
                *flow = ControlFlow::Exit;
            }
            WindowEvent::Resized(size) => {
                render_context.resize(size.into()).unwrap();
            }
            _ => {}
        },
        Event::MainEventsCleared => {
            render_context.render().unwrap();
        }
        _ => {}
    });
}

struct RenderContext {
    renderer: Renderer,
    surface: device::Owned<vk::SurfaceKHR>,
    pipeline_builder: PipelineBuilder,
    pipeline: device::Pipeline,
    vertex_layout: VertexLayout,
    pipeline_layout: device::PipelineLayout,
    start_time: Instant,
    scene: Scene,
}

impl RenderContext {
    fn new(size: (u32, u32), surface: device::Owned<vk::SurfaceKHR>) -> Result<Self> {
        let vertex_layout = renderer::VertexLayout::new()
            .add_vec3(0)
            .add_vec2(1)
            .add_vec4(2);

        let mut compiler = resources::Compiler::new();
        let vs = compiler.compile_vertex(
            r#"
                #version 460
                layout(set = 0, binding = 0) uniform per_draw { mat4 u_mvp; };
                layout(location = 0) in vec3 a_pos;
                layout(location = 1) in vec2 a_uv;
                layout(location = 2) in vec4 a_col;
                layout(location = 0) out vec2 v_uv;
                layout(location = 1) out vec4 v_col;
                void main() {
                    v_uv = a_uv;
                    gl_Position = u_mvp * vec4(a_pos, 1);
                    v_col = a_col;
                }
            "#,
        )?;

        let fs = compiler.compile_fragment(
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
        )?;

        let renderer = Renderer::create(surface.as_raw(), size)?;

        let pipeline_builder = renderer.pipeline_builder()?.add_stage(vs).add_stage(fs);

        let world_set_layout = device::DescriptorSetLayout::builder()
            .add_uniform_buffer(0, vk::ShaderStageFlags::VERTEX)
            .build()?;
        let draw_set_layout = device::DescriptorSetLayout::builder()
            .add_combined_image_sampler(0, vk::ShaderStageFlags::FRAGMENT)
            .build()?;

        let pipeline_layout =
            device::PipelineLayout::create(&[world_set_layout.as_raw(), draw_set_layout.as_raw()])?;

        let pipeline =
            pipeline_builder.build(renderer.size, &vertex_layout, pipeline_layout.as_raw())?;

        let scene = Scene::new(world_set_layout, draw_set_layout)?;

        let start_time = Instant::now();

        Ok(Self {
            renderer,
            surface,
            pipeline_builder,
            pipeline,
            vertex_layout,
            pipeline_layout,
            start_time,
            scene,
        })
    }

    pub fn resize(&mut self, size: (u32, u32)) -> VkResult<()> {
        self.renderer.resize(size)?;
        self.pipeline = self.pipeline_builder.build(
            size,
            &self.vertex_layout,
            self.pipeline_layout.as_raw(),
        )?;
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

        self.scene.render(
            &recorder,
            self.pipeline_layout.as_raw(),
            self.start_time.elapsed(),
        );

        let command_buffer = recorder.end_render_pass().end()?;
        command_buffer.submit_after(
            self.renderer.image_acquire_semaphore.as_raw(),
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        )?;

        self.renderer.present(swapchain_item.index)?;

        Ok(())
    }
}

struct Scene {
    descriptor_pool: device::DescriptorPool,
    world_set: device::DescriptorSet,
    draw_set: device::DescriptorSet,
    mvp_buffer: device::Buffer,
    mesh: resources::Mesh,
    texture: resources::Texture,
}

impl Scene {
    pub fn new(
        world_set_layout: device::DescriptorSetLayout,
        draw_set_layout: device::DescriptorSetLayout,
    ) -> VkResult<Self> {
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

        let world_set = descriptor_pool.allocate(world_set_layout.as_raw())?;
        let draw_set = descriptor_pool.allocate(draw_set_layout.as_raw())?;

        let mvp_buffer = device::Buffer::create(
            device::size_of::<Mat4>(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;

        let texture = resources::Texture::create_2d(128, 128)?;

        let mut upload = texture.begin_upload().unwrap();
        for y in 0..texture.height {
            let row = upload.row(y);
            for x in 0..texture.width {
                let is_stripe = y / 8 % 2 == 0;
                let stripe_y = if is_stripe { y } else { texture.height - y - 1 };
                let a = (x * 256 / texture.width) as u8;
                let v = (stripe_y * a as u32 / texture.height) as u8;
                row[x as usize] = [v, v, v, a];
            }
        }
        upload
            .upload_before(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .unwrap();

        world_set.update_buffer(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            mvp_buffer.as_raw(),
            0,
            64,
        );
        draw_set.update_combined_image_sampler(
            0,
            texture.sampler.as_raw(),
            texture.image_view.as_raw(),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        type Vertex = ([f32; 3], [f32; 2], [f32; 4]);
        let mut vertices: Vec<Vertex> = vec![];
        let mut indices: Vec<u32> = vec![];
        const FACES: &[(Vec3, Vec3)] = &[
            (Vec3::Z_POS, Vec3::Y_POS),
            (Vec3::X_POS, Vec3::Y_POS),
            (Vec3::Z_NEG, Vec3::Y_POS),
            (Vec3::X_NEG, Vec3::Y_POS),
            (Vec3::Y_POS, Vec3::Z_POS),
            (Vec3::Y_NEG, Vec3::Z_POS),
        ];
        for &(out, up) in FACES {
            let right = up.cross(out);
            let bl = out + -up + -right;
            let br = out + -up + right;
            let tl = out + up + -right;
            let tr = out + up + right;

            let index = vertices.len() as u32;

            vertices.push((
                bl.into(),
                [0.0, 0.0],
                Vec4::from((bl + Vec3::ONE) * 0.5).into(),
            ));
            vertices.push((
                br.into(),
                [1.0, 0.0],
                Vec4::from((br + Vec3::ONE) * 0.5).into(),
            ));
            vertices.push((
                tl.into(),
                [0.0, 1.0],
                Vec4::from((tl + Vec3::ONE) * 0.5).into(),
            ));
            vertices.push((
                tr.into(),
                [1.0, 1.0],
                Vec4::from((tr + Vec3::ONE) * 0.5).into(),
            ));
            indices.push(index);
            indices.push(index + 2);
            indices.push(index + 1);
            indices.push(index + 3);
            indices.push(index + 2);
            indices.push(index + 1);
        }

        let mesh = resources::Mesh::create::<Vertex>(&vertices, &indices)?;

        Ok(Self {
            descriptor_pool,
            world_set,
            draw_set,
            mvp_buffer,
            mesh,
            texture,
        })
    }

    fn render(
        &self,
        recorder: &device::CommandBufferRenderPassRecorder,
        pipeline_layout: vk::PipelineLayout,
        elapsed: Duration,
    ) {
        let mvp = Mat4::scale([0.3, 0.4, 0.05].into())
            * Mat4::translate([0.0, 0.0, 2.0].into())
            * Mat4::rotate(Quaternion::axis_angle(
                Vec3::X_POS * std::f32::consts::FRAC_1_SQRT_2
                    + Vec3::Y_POS * std::f32::consts::FRAC_1_SQRT_2,
                elapsed.as_secs_f32(),
            ));
        self.mvp_buffer.write(0, &mvp);
        recorder.bind_descriptor_set(pipeline_layout, 0, self.world_set.as_raw());
        recorder.bind_descriptor_set(pipeline_layout, 1, self.draw_set.as_raw());
        self.mesh.draw(&recorder);
    }
}
