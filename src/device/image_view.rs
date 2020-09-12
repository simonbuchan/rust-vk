use super::*;

impl RawHandle for vk::ImageView {}
impl Create<&vk::ImageViewCreateInfo> for vk::ImageView {
    unsafe fn create(info: &vk::ImageViewCreateInfo) -> VkResult<Self> {
        DEVICE.create_image_view(info, ALLOC)
    }
}
impl Destroy for vk::ImageView {
    unsafe fn destroy(self) {
        DEVICE.destroy_image_view(self, ALLOC)
    }
}

pub struct ImageView(Owned<vk::ImageView>);

impl AsRef<vk::ImageView> for ImageView {
    fn as_ref(&self) -> &vk::ImageView {
        self.0.as_ref()
    }
}

impl ImageView {
    pub fn create_2d(
        image: vk::Image,
        format: vk::Format,
        aspect_mask: vk::ImageAspectFlags,
    ) -> VkResult<Self> {
        Self::create(image, vk::ImageViewType::TYPE_2D, format, aspect_mask)
    }

    pub fn create(
        image: vk::Image,
        view_type: vk::ImageViewType,
        format: vk::Format,
        aspect_mask: vk::ImageAspectFlags,
    ) -> VkResult<Self> {
        let owned = unsafe {
            Owned::create(
                &vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(view_type)
                    .format(format)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .layer_count(vk::REMAINING_ARRAY_LAYERS)
                            .level_count(vk::REMAINING_MIP_LEVELS)
                            .aspect_mask(aspect_mask)
                            .build(),
                    )
                    .build(),
            )?
        };
        Ok(Self(owned))
    }
}
