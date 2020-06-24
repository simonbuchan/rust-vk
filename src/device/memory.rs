use super::*;

pub fn size_of_val<T: ?Sized>(val: &T) -> vk::DeviceSize {
    std::mem::size_of_val(val) as vk::DeviceSize
}

#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct MemoryTypeIndex(pub u32);

impl MemoryTypeIndex {
    const ALL: MemoryTypeIndexAll = MemoryTypeIndexAll(0);

    pub fn is_valid(self) -> bool {
        self.0 < unsafe { &MEMORY_PROPERTIES }.memory_type_count
    }

    pub fn property_flags(self) -> vk::MemoryPropertyFlags {
        if self.is_valid() {
            unsafe { &MEMORY_PROPERTIES }.memory_types[self.0 as usize].property_flags
        } else {
            vk::MemoryPropertyFlags::empty()
        }
    }
}

#[derive(Copy, Clone)]
struct MemoryTypeIndexAll(u32);

impl Iterator for MemoryTypeIndexAll {
    type Item = MemoryTypeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 < unsafe { &MEMORY_PROPERTIES }.memory_type_count {
            let result = Some(MemoryTypeIndex(self.0));
            self.0 += 1;
            result
        } else {
            None
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct MemoryTypeMask(pub u32);

impl MemoryTypeMask {
    pub fn none() -> Self {
        Self(0)
    }

    pub fn any() -> Self {
        Self(!0)
    }

    pub fn from_index(index: MemoryTypeIndex) -> Self {
        Self(1 << index.0)
    }

    pub fn with_properties(flags: vk::MemoryPropertyFlags) -> Self {
        let mut mask = Self::none();
        for index in MemoryTypeIndex::ALL {
            if index.property_flags().contains(flags) {
                mask |= Self::from_index(index);
            }
        }
        mask
    }

    pub fn mappable() -> MemoryTypeMask {
        Self::with_properties(
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
    }

    pub fn first_index(self) -> MemoryTypeIndex {
        MemoryTypeIndex(self.0.trailing_zeros())
    }
}

impl std::ops::BitAnd for MemoryTypeMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for MemoryTypeMask {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}

impl std::ops::BitOr for MemoryTypeMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for MemoryTypeMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl RawHandle for vk::DeviceMemory {}
impl Create<&vk::MemoryAllocateInfo> for vk::DeviceMemory {
    unsafe fn create(info: &vk::MemoryAllocateInfo) -> VkResult<Self> {
        DEVICE.allocate_memory(info, ALLOC)
    }
}
impl Destroy for vk::DeviceMemory {
    unsafe fn destroy(self) {
        DEVICE.free_memory(self, ALLOC)
    }
}

pub struct Memory(Owned<vk::DeviceMemory>);

impl AsRef<vk::DeviceMemory> for Memory {
    fn as_ref(&self) -> &vk::DeviceMemory {
        self.0.as_ref()
    }
}

impl Memory {
    pub fn allocate_mappable(size: vk::DeviceSize, type_mask: MemoryTypeMask) -> VkResult<Memory> {
        Self::allocate(size, (type_mask & MemoryTypeMask::mappable()).first_index())
    }

    pub fn allocate(size: vk::DeviceSize, type_index: MemoryTypeIndex) -> VkResult<Memory> {
        let owner = unsafe {
            Owned::create(
                &vk::MemoryAllocateInfo::builder()
                    .allocation_size(size)
                    .memory_type_index(type_index.0)
                    .build(),
            )?
        };
        Ok(Self(owner))
    }

    pub fn map(&self, offset: vk::DeviceSize, size: usize) -> VkResult<MemoryMapping> {
        unsafe {
            let ptr = DEVICE.map_memory(
                self.as_raw(),
                offset,
                size as vk::DeviceSize,
                vk::MemoryMapFlags::empty(),
            )?;
            Ok(MemoryMapping {
                ptr,
                size,
                device_memory: self.as_raw(),
            })
        }
    }

    pub fn write<T: Copy + ?Sized>(&self, offset: vk::DeviceSize, source: &T) -> VkResult<()> {
        let mut mapping = self.map(offset, std::mem::size_of_val(source))?;
        mapping.write(0, source);
        Ok(())
    }

    pub fn write_slice<T: Copy>(&self, offset: vk::DeviceSize, source: &[T]) -> VkResult<()> {
        let mut mapping = self.map(offset, std::mem::size_of_val(source))?;
        mapping.write_slice(0, source);
        Ok(())
    }
}

pub struct MemoryMapping {
    ptr: *mut c_void,
    size: usize,
    device_memory: vk::DeviceMemory,
}

impl Drop for MemoryMapping {
    fn drop(&mut self) {
        unsafe { DEVICE.unmap_memory(self.device_memory) }
    }
}

impl MemoryMapping {
    pub fn write<T: Copy + ?Sized>(&mut self, offset: usize, src: &T) {
        let len = std::mem::size_of_val(src);
        assert!(offset + len <= self.size);
        let ptr = unsafe { self.ptr.add(offset) };
        unsafe { std::ptr::copy_nonoverlapping(src as *const T as *const c_void, ptr, len) };
    }

    pub fn slice<T: Copy>(&mut self, offset: usize, len: usize) -> &mut [T] {
        assert!(offset + len <= self.size);
        let ptr = unsafe { self.ptr.add(offset) }.cast();
        unsafe { std::slice::from_raw_parts_mut::<T>(ptr, len) }
    }

    pub fn write_slice<T: Copy>(&mut self, offset: usize, source: &[T]) {
        self.slice::<T>(offset, source.len())
            .copy_from_slice(source);
    }
}
