use crate::{device::*, globals::*};

pub struct Renderer {
    pub surface: vk::SurfaceKHR,
    pub render_pass: Owned<vk::RenderPass>,
    pub size: (u32, u32),
    pub color_format: vk::Format,
    pub samples: vk::SampleCountFlags,
    pub image_acquire_semaphore: Semaphore,
    swapchain: Swapchain,
}

fn get_format_and_samples(_surface: vk::SurfaceKHR) -> Result<(vk::Format, vk::SampleCountFlags)> {
    unsafe {
        // let surface_formats =
        //     ext::SURFACE.get_physical_device_surface_formats(PHYSICAL_DEVICE, surface)?;
        //
        let format = vk::Format::B8G8R8A8_SRGB;

        let props = INSTANCE.get_physical_device_image_format_properties(
            PHYSICAL_DEVICE,
            format,
            vk::ImageType::TYPE_2D,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::ImageCreateFlags::empty(),
        )?;

        let sample_count_flags =
            vk::SampleCountFlags::from_raw(props.sample_counts.as_raw().next_power_of_two() >> 1);
        return Ok((format, sample_count_flags));
    }
    // Err(Error::VkSupport)
}

impl Renderer {
    pub fn create(surface: vk::SurfaceKHR, size: (u32, u32)) -> Result<Self> {
        unsafe {
            let (color_format, samples) = get_format_and_samples(surface)?;
            println!("surface format: {:?}, samples: {:?}", color_format, samples);

            let render_pass = Owned::<vk::RenderPass>::create(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&[
                        vk::AttachmentDescription::builder()
                            .format(color_format)
                            .samples(samples)
                            .load_op(vk::AttachmentLoadOp::CLEAR)
                            .store_op(vk::AttachmentStoreOp::STORE)
                            .initial_layout(vk::ImageLayout::UNDEFINED)
                            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .build(),
                        vk::AttachmentDescription::builder()
                            .format(vk::Format::D32_SFLOAT)
                            .samples(samples)
                            .load_op(vk::AttachmentLoadOp::CLEAR)
                            .store_op(vk::AttachmentStoreOp::DONT_CARE)
                            .initial_layout(vk::ImageLayout::UNDEFINED)
                            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                            .build(),
                        vk::AttachmentDescription::builder()
                            .format(color_format)
                            .samples(vk::SampleCountFlags::TYPE_1)
                            .load_op(vk::AttachmentLoadOp::DONT_CARE)
                            .store_op(vk::AttachmentStoreOp::STORE)
                            .initial_layout(vk::ImageLayout::UNDEFINED)
                            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                            .build(),
                    ])
                    .subpasses(&[vk::SubpassDescription::builder()
                        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                        .color_attachments(&[vk::AttachmentReference::builder()
                            .attachment(0)
                            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .build()])
                        .depth_stencil_attachment(
                            &vk::AttachmentReference::builder()
                                .attachment(1)
                                .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                                .build(),
                        )
                        .resolve_attachments(&[vk::AttachmentReference::builder()
                            .attachment(2)
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
                samples,
            )?;

            Ok(Self {
                surface,
                render_pass,
                size,
                color_format,
                samples,
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
                self.samples,
            )?;
            self.size = size;
        }
        Ok(())
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

#[allow(dead_code)]
pub struct Swapchain {
    swapchain: Owned<vk::SwapchainKHR>,
    depth_image: Image,
    depth_image_view: ImageView,
    color_image: Image,
    color_image_view: ImageView,
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
        samples: vk::SampleCountFlags,
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

            let depth_image = Image::create_2d(
                (width, height),
                1,
                vk::Format::D32_SFLOAT,
                samples,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                MemoryTypeMask::any(),
            )?;
            let depth_image_view = ImageView::create_2d(
                depth_image.object.as_raw(),
                vk::Format::D32_SFLOAT,
                vk::ImageAspectFlags::DEPTH,
            )?;

            let color_image = Image::create_2d(
                (width, height),
                1,
                color_format,
                samples,
                vk::ImageUsageFlags::COLOR_ATTACHMENT,
                MemoryTypeMask::any(),
            )?;
            let color_image_view = ImageView::create_2d(
                color_image.object.as_raw(),
                color_format,
                vk::ImageAspectFlags::COLOR,
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
                            .attachments(&[
                                color_image_view.as_raw(),
                                depth_image_view.as_raw(),
                                image_view.as_raw(),
                            ])
                            .width(width)
                            .height(height)
                            .layers(1)
                            .build(),
                    )
                })
                .collect::<VkResult<Vec<_>>>()?;

            Ok(Self {
                swapchain,
                depth_image,
                depth_image_view,
                color_image,
                color_image_view,
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
        samples: vk::SampleCountFlags,
    ) -> VkResult<()> {
        *self = Self::create(
            self.swapchain.as_raw(),
            surface,
            render_pass,
            (width, height),
            color_format,
            samples,
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
