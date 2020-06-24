use super::*;

impl RawHandle for vk::Buffer {}
impl Create<&vk::BufferCreateInfo> for vk::Buffer {
    unsafe fn create(info: &vk::BufferCreateInfo) -> VkResult<Self> {
        DEVICE.create_buffer(info, ALLOC)
    }
}
impl Destroy for vk::Buffer {
    unsafe fn destroy(self) {
        DEVICE.destroy_buffer(self, ALLOC)
    }
}

pub struct BufferObject(Owned<vk::Buffer>);

impl AsRef<vk::Buffer> for BufferObject {
    fn as_ref(&self) -> &vk::Buffer {
        self.0.as_ref()
    }
}

impl BufferObject {
    pub fn create(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> VkResult<Self> {
        Ok(Self(unsafe {
            Owned::create(
                &vk::BufferCreateInfo::builder()
                    .size(size)
                    .usage(usage)
                    .queue_family_indices(&[GRAPHICS_QUEUE_FAMILY_INDEX])
                    .build(),
            )?
        }))
    }

    pub fn memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe { DEVICE.get_buffer_memory_requirements(self.as_raw()) }
    }

    pub fn bind_memory(
        &self,
        memory: impl AsRawHandle<vk::DeviceMemory>,
        offset: vk::DeviceSize,
    ) -> VkResult<()> {
        unsafe { DEVICE.bind_buffer_memory(self.as_raw(), memory.as_raw(), offset) }
    }
}

pub struct Buffer {
    pub object: BufferObject,
    pub memory: Memory,
}

impl AsRef<vk::DeviceMemory> for Buffer {
    fn as_ref(&self) -> &vk::DeviceMemory {
        self.memory.as_ref()
    }
}

impl AsRef<vk::Buffer> for Buffer {
    fn as_ref(&self) -> &vk::Buffer {
        self.object.as_ref()
    }
}

impl Buffer {
    pub fn create_with<T: Copy + ?Sized>(
        usage: vk::BufferUsageFlags,
        source: &T,
    ) -> VkResult<Self> {
        let buffer = Self::create(std::mem::size_of_val(source) as vk::DeviceSize, usage)?;
        buffer.write(0, source)?;
        Ok(buffer)
    }

    pub fn create(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> VkResult<Self> {
        let buffer = BufferObject::create(size, usage)?;
        let memory_requirements = buffer.memory_requirements();

        let memory = Memory::allocate_mappable(
            memory_requirements.size,
            MemoryTypeMask(memory_requirements.memory_type_bits),
        )?;

        buffer.bind_memory(&memory, 0)?;

        Ok(Self {
            object: buffer,
            memory,
        })
    }

    pub fn write<T: Copy + ?Sized>(&self, offset: vk::DeviceSize, source: &T) -> VkResult<()> {
        self.memory.write(offset, source)
    }
}
