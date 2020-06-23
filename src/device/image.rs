use super::*;

impl RawHandle for vk::Image {}
impl Create<&vk::ImageCreateInfo> for vk::Image {
    unsafe fn create(info: &vk::ImageCreateInfo) -> VkResult<Self> {
        DEVICE.create_image(info, ALLOC)
    }
}
impl Destroy for vk::Image {
    unsafe fn destroy(self) {
        DEVICE.destroy_image(self, ALLOC)
    }
}

pub struct ImageObject(Owned<vk::Image>);

impl AsRef<vk::Image> for ImageObject {
    fn as_ref(&self) -> &vk::Image {
        self.0.as_ref()
    }
}

impl ImageObject {
    pub fn create_2d(
        (width, height): (u32, u32),
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        inital_layout: vk::ImageLayout,
    ) -> VkResult<Self> {
        unsafe {
            let owned = Owned::create(
                &vk::ImageCreateInfo::builder()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(format)
                    .extent(vk::Extent3D {
                        width,
                        height,
                        depth: 1,
                    })
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(tiling)
                    .usage(usage)
                    .queue_family_indices(&[GRAPHICS_QUEUE_FAMILY_INDEX])
                    .initial_layout(inital_layout)
                    .build(),
            )?;

            Ok(Self(owned))
        }
    }

    pub fn memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe { DEVICE.get_image_memory_requirements(self.as_raw()) }
    }

    pub fn bind_memory(&self, memory: vk::DeviceMemory) -> VkResult<()> {
        unsafe { DEVICE.bind_image_memory(self.as_raw(), memory, 0) }
    }

    pub fn color_subresource_layout(&self) -> vk::SubresourceLayout {
        unsafe {
            DEVICE.get_image_subresource_layout(
                self.as_raw(),
                vk::ImageSubresource::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .build(),
            )
        }
    }
}

pub struct Image {
    pub object: ImageObject,
    pub memory: Memory,
}

impl Image {
    pub fn create_2d(
        size: (u32, u32),
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        initial_layout: vk::ImageLayout,
        memory_type_mask: MemoryTypeMask,
    ) -> VkResult<Self> {
        let image =
            ImageObject::create_2d(size, format, vk::ImageTiling::LINEAR, usage, initial_layout)?;
        let memory_requirements = image.memory_requirements();

        let memory = Memory::allocate_mappable(
            memory_requirements.size,
            MemoryTypeMask(memory_requirements.memory_type_bits) & memory_type_mask,
        )?;

        image.bind_memory(memory.as_raw())?;

        Ok(Self {
            object: image,
            memory,
        })
    }
}
