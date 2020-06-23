use super::*;

impl RawHandle for vk::Fence {}
impl Create<&vk::FenceCreateInfo> for vk::Fence {
    unsafe fn create(info: &vk::FenceCreateInfo) -> VkResult<Self> {
        DEVICE.create_fence(info, ALLOC)
    }
}
impl Destroy for vk::Fence {
    unsafe fn destroy(self) {
        DEVICE.destroy_fence(self, ALLOC)
    }
}

pub struct Fence(Owned<vk::Fence>);

impl AsRef<vk::Fence> for Fence {
    fn as_ref(&self) -> &vk::Fence {
        self.0.as_ref()
    }
}

impl Fence {
    pub fn create() -> VkResult<Self> {
        Ok(Self(unsafe {
            Owned::create(&vk::FenceCreateInfo::default())?
        }))
    }

    pub fn create_signalled() -> VkResult<Self> {
        Ok(Self(unsafe {
            Owned::create(
                &vk::FenceCreateInfo::builder()
                    .flags(vk::FenceCreateFlags::SIGNALED)
                    .build(),
            )?
        }))
    }

    pub fn is_signalled(&self) -> VkResult<bool> {
        unsafe { DEVICE.get_fence_status(self.as_raw()) }
    }

    pub fn wait(&self) -> VkResult<()> {
        unsafe { DEVICE.wait_for_fences(&[self.as_raw()], false, !0) }
    }

    pub fn reset(&self) -> VkResult<()> {
        unsafe { DEVICE.reset_fences(&[self.as_raw()]) }
    }
}
