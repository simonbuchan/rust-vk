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

pub fn mip_levels((width, height): (u32, u32)) -> u32 {
    // If the image has a max dimension of 5=0b101,
    // then the mip levels and their max dimension are:
    //   0 = 0b101,  1 = 0b10,   2 = 0b1
    // For 32:
    //   0 = 0b10_0000,  1 = 0b1_0000
    //   2 =    0b1000,  3 =    0b100
    //   4 =      0b10,  5 =      0b1
    // So essentially find the position of the top set bit:
    32 - width.max(height).leading_zeros()
}

impl ImageObject {
    pub fn create_2d(
        (width, height): (u32, u32),
        mip_levels: u32,
        format: vk::Format,
        samples: vk::SampleCountFlags,
        usage: vk::ImageUsageFlags,
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
                    .mip_levels(mip_levels)
                    .array_layers(1)
                    .samples(samples)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(usage)
                    .queue_family_indices(&[GRAPHICS_QUEUE_FAMILY_INDEX])
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .build(),
            )?;

            Ok(Self(owned))
        }
    }

    pub fn create_cube(
        (width, height): (u32, u32),
        mip_levels: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> VkResult<Self> {
        unsafe {
            let owned = Owned::create(
                &vk::ImageCreateInfo::builder()
                    .flags(vk::ImageCreateFlags::CUBE_COMPATIBLE)
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(format)
                    .extent(vk::Extent3D {
                        width,
                        height,
                        depth: 1,
                    })
                    .mip_levels(mip_levels)
                    .array_layers(6)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(usage)
                    .queue_family_indices(&[GRAPHICS_QUEUE_FAMILY_INDEX])
                    .initial_layout(vk::ImageLayout::UNDEFINED)
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
}

pub struct Image {
    pub object: ImageObject,
    pub memory: Memory,
}

impl Image {
    pub fn create_2d(
        size: (u32, u32),
        mip_levels: u32,
        format: vk::Format,
        samples: vk::SampleCountFlags,
        usage: vk::ImageUsageFlags,
        memory_type_mask: MemoryTypeMask,
    ) -> VkResult<Self> {
        Self::create(
            ImageObject::create_2d(size, mip_levels, format, samples, usage)?,
            memory_type_mask,
        )
    }

    pub fn create(object: ImageObject, memory_type_mask: MemoryTypeMask) -> VkResult<Self> {
        let memory_requirements = object.memory_requirements();

        let memory = Memory::allocate(
            memory_requirements.size,
            (MemoryTypeMask(memory_requirements.memory_type_bits) & memory_type_mask).first_index(),
        )?;

        object.bind_memory(memory.as_raw())?;

        Ok(Self { object, memory })
    }
}
