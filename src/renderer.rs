use std::ffi::CStr;

use crate::resources::Shader;
use crate::{device::*, globals::*};

pub struct Renderer {
    pub surface: vk::SurfaceKHR,
    pub render_pass: Owned<vk::RenderPass>,
    pub size: (u32, u32),
    pub color_format: vk::Format,
    pub image_acquire_semaphore: Semaphore,
    swapchain: Swapchain,
}

impl Renderer {
    pub fn create(surface: vk::SurfaceKHR, size: (u32, u32)) -> VkResult<Self> {
        unsafe {
            let color_format = vk::Format::B8G8R8A8_UNORM;

            let render_pass = Owned::<vk::RenderPass>::create(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&[vk::AttachmentDescription::builder()
                        .format(color_format)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .initial_layout(vk::ImageLayout::UNDEFINED)
                        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .build()])
                    .subpasses(&[vk::SubpassDescription::builder()
                        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                        .color_attachments(&[vk::AttachmentReference::builder()
                            .attachment(0)
                            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .build()])
                        .build()])
                    .build(),
            )?;

            let image_acquire_semaphore = Semaphore::create()?;

            let swapchain = Swapchain::create(
                vk::SwapchainKHR::null(),
                surface,
                render_pass.as_raw(),
                size,
                color_format,
            )?;

            Ok(Self {
                surface,
                render_pass,
                size,
                color_format,
                image_acquire_semaphore,
                swapchain,
            })
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) -> VkResult<()> {
        if self.size != size {
            self.swapchain.update(
                self.surface,
                self.render_pass.as_raw(),
                size,
                self.color_format,
            )?;
            self.size = size;
        }
        Ok(())
    }

    pub fn pipeline_builder(&self) -> VkResult<PipelineBuilder> {
        PipelineBuilder::new(self.render_pass.as_raw())
    }

    pub fn acquire_image(&mut self) -> VkResult<SwapchainItem> {
        self.swapchain.next(
            u64::max_value(),
            self.image_acquire_semaphore.as_raw(),
            vk::Fence::null(),
        )
    }

    pub fn present(&mut self, index: u32) -> VkResult<()> {
        unsafe {
            let mut swapchain_results = [vk::Result::SUCCESS];
            let _suboptimal = ext::SWAPCHAIN.queue_present(
                GRAPHICS_QUEUE,
                &vk::PresentInfoKHR::builder()
                    .swapchains(&[self.swapchain.swapchain.as_raw()])
                    .image_indices(&[index])
                    .results(&mut swapchain_results)
                    .build(),
            )?;
            if swapchain_results[0] != vk::Result::SUCCESS {
                return Err(swapchain_results[0]);
            }
            Ok(())
        }
    }
}

pub struct Swapchain {
    swapchain: Owned<vk::SwapchainKHR>,
    images: Vec<vk::Image>,
    image_views: Vec<ImageView>,
    framebuffers: Vec<Owned<vk::Framebuffer>>,
}

pub struct SwapchainItem {
    pub index: u32,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub framebuffer: vk::Framebuffer,
}

impl Swapchain {
    pub fn create(
        old_swapchain: vk::SwapchainKHR,
        surface: vk::SurfaceKHR,
        render_pass: vk::RenderPass,
        (width, height): (u32, u32),
        color_format: vk::Format,
    ) -> VkResult<Self> {
        unsafe {
            let swapchain = Owned::create(
                &vk::SwapchainCreateInfoKHR::builder()
                    .surface(surface)
                    .min_image_count(2)
                    .image_format(color_format)
                    .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                    .image_extent(vk::Extent2D { width, height })
                    .image_array_layers(1)
                    .image_usage(
                        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                    )
                    .queue_family_indices(&[GRAPHICS_QUEUE_FAMILY_INDEX])
                    .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(vk::PresentModeKHR::FIFO)
                    .old_swapchain(old_swapchain)
                    .build(),
            )?;

            let images = ext::SWAPCHAIN.get_swapchain_images(swapchain.as_raw())?;

            let image_views = images
                .iter()
                .map(|&image| {
                    ImageView::create_2d(image, color_format, vk::ImageAspectFlags::COLOR)
                })
                .collect::<VkResult<Vec<_>>>()?;

            let framebuffers = image_views
                .iter()
                .map(|image_view| {
                    Owned::create(
                        &vk::FramebufferCreateInfo::builder()
                            .render_pass(render_pass)
                            .attachments(&[image_view.as_raw()])
                            .width(width)
                            .height(height)
                            .layers(1)
                            .build(),
                    )
                })
                .collect::<VkResult<Vec<_>>>()?;

            Ok(Self {
                swapchain,
                images,
                image_views,
                framebuffers,
            })
        }
    }

    pub fn update(
        &mut self,
        surface: vk::SurfaceKHR,
        render_pass: vk::RenderPass,
        (width, height): (u32, u32),
        color_format: vk::Format,
    ) -> VkResult<()> {
        *self = Self::create(
            self.swapchain.as_raw(),
            surface,
            render_pass,
            (width, height),
            color_format,
        )?;

        Ok(())
    }

    pub fn next(
        &self,
        timeout: u64,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> VkResult<SwapchainItem> {
        unsafe {
            let (index, _suboptimal) = ext::SWAPCHAIN.acquire_next_image(
                self.swapchain.as_raw(),
                timeout,
                semaphore,
                fence,
            )?;
            Ok(SwapchainItem {
                index,
                image: self.images[index as usize],
                image_view: self.image_views[index as usize].as_raw(),
                framebuffer: self.framebuffers[index as usize].as_raw(),
            })
        }
    }
}

pub struct VertexLayout {
    input_attributes: Vec<vk::VertexInputAttributeDescription>,
    size: u32,
}

#[allow(dead_code)]
impl VertexLayout {
    pub fn new() -> Self {
        Self {
            input_attributes: vec![],
            size: 0,
        }
    }

    pub fn add_float(self, location: u32) -> Self {
        self.add(location, vk::Format::R32_SFLOAT, 4)
    }

    pub fn add_vec2(self, location: u32) -> Self {
        self.add(location, vk::Format::R32G32_SFLOAT, 8)
    }

    pub fn add_vec3(self, location: u32) -> Self {
        self.add(location, vk::Format::R32G32B32_SFLOAT, 12)
    }

    pub fn add_vec4(self, location: u32) -> Self {
        self.add(location, vk::Format::R32G32B32A32_SFLOAT, 16)
    }

    fn add(mut self, location: u32, format: vk::Format, size: u32) -> Self {
        self.input_attributes.push(
            vk::VertexInputAttributeDescription::builder()
                .location(location)
                .offset(self.size)
                .binding(0)
                .format(format)
                .build(),
        );
        self.size += size;
        self
    }
}

pub struct PipelineBuilder {
    render_pass: vk::RenderPass,
    stages: Vec<Shader>,
    cache: PipelineCache,
}

impl PipelineBuilder {
    pub fn new(render_pass: vk::RenderPass) -> VkResult<Self> {
        Ok(Self {
            render_pass,
            stages: vec![],
            cache: PipelineCache::create()?,
        })
    }

    pub fn add_stage(mut self, shader: Shader) -> Self {
        self.stages.push(shader);
        self
    }

    pub fn build(
        &self,
        (width, height): (u32, u32),
        vertex_layout: &VertexLayout,
        layout: vk::PipelineLayout,
    ) -> VkResult<Pipeline> {
        self.cache.create_pipeline(
            &vk::GraphicsPipelineCreateInfo::builder()
                .stages(
                    &self
                        .stages
                        .iter()
                        .map(|stage| {
                            vk::PipelineShaderStageCreateInfo::builder()
                                .name(&CStr::from_bytes_with_nul(b"main\0").unwrap())
                                .module(stage.as_raw())
                                .stage(stage.stage())
                                .build()
                        })
                        .collect::<Vec<_>>(),
                )
                .vertex_input_state(
                    &vk::PipelineVertexInputStateCreateInfo::builder()
                        .vertex_attribute_descriptions(&vertex_layout.input_attributes)
                        .vertex_binding_descriptions(&[vk::VertexInputBindingDescription::builder(
                        )
                        .binding(0)
                        .stride(vertex_layout.size)
                        .input_rate(vk::VertexInputRate::VERTEX)
                        .build()])
                        .build(),
                )
                .input_assembly_state(
                    &vk::PipelineInputAssemblyStateCreateInfo::builder()
                        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                        .build(),
                )
                .viewport_state(
                    &vk::PipelineViewportStateCreateInfo::builder()
                        .viewports(&[vk::Viewport::builder()
                            .width(width as f32)
                            .height(height as f32)
                            .min_depth(0.0)
                            .max_depth(1.0)
                            .build()])
                        .scissors(&[vk::Rect2D::builder()
                            .extent(vk::Extent2D { width, height })
                            .build()])
                        .build(),
                )
                .rasterization_state(
                    &vk::PipelineRasterizationStateCreateInfo::builder()
                        .line_width(1.0)
                        .build(),
                )
                .multisample_state(
                    &vk::PipelineMultisampleStateCreateInfo::builder()
                        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
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
                .layout(layout)
                .render_pass(self.render_pass)
                .subpass(0)
                .build(),
        )
    }
}
