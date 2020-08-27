use crate::device::{self, AsRawHandle};
use ash::{prelude::*, vk};

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub image: device::Image,
    pub image_view: device::ImageView,
    pub sampler: device::Sampler,
}

impl Texture {
    pub fn create(width: u32, height: u32, format: vk::Format) -> VkResult<Texture> {
        let image = device::Image::create_2d(
            (width, height),
            device::mip_levels((width, height)),
            format,
            vk::SampleCountFlags::TYPE_1,
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED,
            device::MemoryTypeMask::any(),
        )?;

        let image_view = device::ImageView::create_2d(
            image.object.as_raw(),
            format,
            vk::ImageAspectFlags::COLOR,
        )?;

        let sampler = device::Sampler::linear()?;

        Ok(Self {
            width,
            height,
            image,
            image_view,
            sampler,
        })
    }

    pub fn copy_from(&self, buffer: vk::Buffer, offset: vk::DeviceSize) -> VkResult<()> {
        let recording = device::CommandBuffer::create()?;
        // Transition to be a transfer target
        recording.image_transition(
            vk::PipelineStageFlags::HOST,
            vk::PipelineStageFlags::TRANSFER,
            &[vk::ImageMemoryBarrier::builder()
                .image(self.image.object.as_raw())
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(vk::REMAINING_ARRAY_LAYERS)
                        .level_count(vk::REMAINING_MIP_LEVELS)
                        .build(),
                )
                .build()],
        );
        // Copy image data from buffer to first mip level. Assume buffer data is packed.
        recording.copy_buffer_to_image(
            buffer,
            self.image.object.as_raw(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[vk::BufferImageCopy::builder()
                .buffer_offset(offset)
                .image_subresource(
                    vk::ImageSubresourceLayers::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(1)
                        .mip_level(0)
                        .build(),
                )
                .image_extent(vk::Extent3D {
                    width: self.width,
                    height: self.height,
                    depth: 1,
                })
                .build()],
        );
        // Now transition first mip level to transfer source
        recording.image_transition(
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            &[vk::ImageMemoryBarrier::builder()
                .image(self.image.object.as_raw())
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(vk::REMAINING_ARRAY_LAYERS)
                        .level_count(1)
                        .build(),
                )
                .build()],
        );

        // And blit it to remaining mip levels
        recording.blit_image(
            self.image.object.as_raw(),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            self.image.object.as_raw(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &(1..device::mip_levels((self.width, self.height)))
                .into_iter()
                .map(|level| {
                    vk::ImageBlit::builder()
                        .src_offsets([
                            vk::Offset3D::default(),
                            vk::Offset3D {
                                x: self.width as i32,
                                y: self.height as i32,
                                z: 1,
                            },
                        ])
                        .src_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .layer_count(1)
                                .mip_level(0)
                                .build(),
                        )
                        .dst_offsets([
                            vk::Offset3D::default(),
                            vk::Offset3D {
                                x: 1.max(self.width as i32 >> level),
                                y: 1.max(self.height as i32 >> level),
                                z: 1,
                            },
                        ])
                        .dst_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .layer_count(1)
                                .mip_level(level)
                                .build(),
                        )
                        .build()
                })
                .collect::<Vec<vk::ImageBlit>>(),
            vk::Filter::LINEAR,
        );
        recording.image_transition(
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            &[
                vk::ImageMemoryBarrier::builder()
                    .image(self.image.object.as_raw())
                    .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
                    .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(vk::REMAINING_ARRAY_LAYERS)
                            .level_count(1)
                            .build(),
                    )
                    .build(),
                vk::ImageMemoryBarrier::builder()
                    .image(self.image.object.as_raw())
                    .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(vk::REMAINING_ARRAY_LAYERS)
                            .base_mip_level(1)
                            .level_count(vk::REMAINING_MIP_LEVELS)
                            .build(),
                    )
                    .build(),
            ],
        );
        recording.end()?.submit()?;
        Ok(())
    }
}
