use super::*;

impl RawHandle for vk::Semaphore {}
impl Create<&vk::SemaphoreCreateInfo> for vk::Semaphore {
    unsafe fn create(info: &vk::SemaphoreCreateInfo) -> VkResult<Self> {
        DEVICE.create_semaphore(info, ALLOC)
    }
}
impl Destroy for vk::Semaphore {
    unsafe fn destroy(self) {
        DEVICE.destroy_semaphore(self, ALLOC)
    }
}

pub struct Semaphore(Owned<vk::Semaphore>);

impl AsRef<vk::Semaphore> for Semaphore {
    fn as_ref(&self) -> &vk::Semaphore {
        self.0.as_ref()
    }
}

impl Semaphore {
    pub fn create() -> VkResult<Self> {
        Ok(Self(unsafe {
            Owned::create(&vk::SemaphoreCreateInfo::default())?
        }))
    }
}
