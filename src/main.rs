use std::time::Instant;

use ash::prelude::VkResult;
use ash::vk;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use device::{AsRawHandle, CommandBuffer};
use error::*;

mod device;
// mod ecs;
mod error;
mod globals;
mod init;
mod math;
// mod render_graph;
mod renderer;
mod resources;
mod scene;

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
            render_context.update();
            render_context.render().unwrap();
        }
        _ => {}
    });
}

#[allow(dead_code)]
struct RenderContext {
    renderer: renderer::Renderer,
    surface: device::Owned<vk::SurfaceKHR>,
    start_time: Instant,
    scene: scene::SceneWatcher,
}

impl RenderContext {
    fn new(size: (u32, u32), surface: device::Owned<vk::SurfaceKHR>) -> Result<Self> {
        let renderer = renderer::Renderer::create(surface.as_raw(), size)?;
        let mut scene = scene::SceneWatcher::create(
            renderer.render_pass.as_raw(),
            renderer.samples,
            std::path::Path::new("pipeline.scene"),
        )?;
        scene.resize(size);

        let start_time = Instant::now();

        Ok(Self {
            renderer,
            surface,
            start_time,
            scene,
        })
    }

    pub fn resize(&mut self, size: (u32, u32)) -> VkResult<()> {
        self.renderer.resize(size)?;
        self.scene.resize(size);
        Ok(())
    }

    pub fn update(&mut self) {
        self.scene.update(self.start_time.elapsed());
    }

    pub fn render(&mut self) -> Result<()> {
        use globals::*;
        let swapchain_item = self.renderer.acquire_image()?;
        let (width, height) = self.renderer.size;

        let recorder = CommandBuffer::create()?;
        recorder.set_viewport_scissor(self.renderer.size);

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

        self.scene.render(&recorder)?;

        let command_buffer = recorder.end_render_pass().end()?;
        command_buffer.submit_after(
            self.renderer.image_acquire_semaphore.as_raw(),
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        )?;

        self.renderer.present(swapchain_item.index)?;

        Ok(())
    }
}
