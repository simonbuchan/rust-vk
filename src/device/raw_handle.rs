use crate::globals::*;

pub trait Create<Info>: Sized {
    unsafe fn create(info: Info) -> VkResult<Self>;
}
pub trait CreateVec<Info>: Sized {
    unsafe fn create_vec(info: Info) -> VkResult<Vec<Self>>;
}

pub trait Destroy: Sized + Copy {
    unsafe fn destroy(self);
}

pub trait RawHandle: Copy {}

pub trait AsRawHandle<T: RawHandle> {
    fn as_raw(&self) -> T;
}

impl<T: RawHandle, R: AsRef<T>> AsRawHandle<T> for R {
    fn as_raw(&self) -> T {
        *self.as_ref()
    }
}

pub struct Owned<T: Destroy>(T);

impl<T: Destroy> AsRef<T> for Owned<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: RawHandle + Destroy> Owned<T> {
    pub unsafe fn from_raw(handle: T) -> Self {
        Self(handle)
    }

    pub unsafe fn into_raw(self) -> T {
        let raw = self.0;
        std::mem::forget(self);
        raw
    }
}

impl<T: Destroy> Owned<T> {
    pub unsafe fn create<I>(info: I) -> VkResult<Self>
    where
        T: Create<I>,
    {
        Ok(Self(T::create(info)?))
    }

    pub unsafe fn create_vec<I>(info: I) -> VkResult<Vec<Self>>
    where
        T: CreateVec<I>,
    {
        let raw_vec = T::create_vec(info)?;
        Ok(raw_vec.into_iter().map(Self).collect())
    }
}

impl<T: Destroy> Drop for Owned<T> {
    fn drop(&mut self) {
        unsafe { self.0.destroy() }
    }
}

impl RawHandle for vk::RenderPass {}
impl Create<&vk::RenderPassCreateInfo> for vk::RenderPass {
    unsafe fn create(info: &vk::RenderPassCreateInfo) -> VkResult<Self> {
        DEVICE.create_render_pass(info, ALLOC)
    }
}
impl Create<&vk::RenderPassCreateInfo2> for vk::RenderPass {
    unsafe fn create(info: &vk::RenderPassCreateInfo2) -> VkResult<Self> {
        DEVICE.create_render_pass2(info, ALLOC)
    }
}
impl Destroy for vk::RenderPass {
    unsafe fn destroy(self) {
        DEVICE.destroy_render_pass(self, ALLOC)
    }
}

impl RawHandle for vk::BufferView {}
impl Create<&vk::BufferViewCreateInfo> for vk::BufferView {
    unsafe fn create(info: &vk::BufferViewCreateInfo) -> VkResult<Self> {
        DEVICE.create_buffer_view(info, ALLOC)
    }
}
impl Destroy for vk::BufferView {
    unsafe fn destroy(self) {
        DEVICE.destroy_buffer_view(self, ALLOC)
    }
}

impl RawHandle for vk::SurfaceKHR {}
impl Create<&vk::Win32SurfaceCreateInfoKHR> for vk::SurfaceKHR {
    unsafe fn create(info: &vk::Win32SurfaceCreateInfoKHR) -> VkResult<Self> {
        ext::WIN32_SURFACE.create_win32_surface(info, ALLOC)
    }
}
impl Destroy for vk::SurfaceKHR {
    unsafe fn destroy(self) {
        ext::SURFACE.destroy_surface(self, ALLOC)
    }
}

impl RawHandle for vk::SwapchainKHR {}
impl Create<&vk::SwapchainCreateInfoKHR> for vk::SwapchainKHR {
    unsafe fn create(info: &vk::SwapchainCreateInfoKHR) -> VkResult<Self> {
        ext::SWAPCHAIN.create_swapchain(info, ALLOC)
    }
}
impl Destroy for vk::SwapchainKHR {
    unsafe fn destroy(self) {
        ext::SWAPCHAIN.destroy_swapchain(self, ALLOC)
    }
}

impl RawHandle for vk::ShaderModule {}
impl Create<&vk::ShaderModuleCreateInfo> for vk::ShaderModule {
    unsafe fn create(info: &vk::ShaderModuleCreateInfo) -> VkResult<Self> {
        DEVICE.create_shader_module(info, ALLOC)
    }
}
impl Destroy for vk::ShaderModule {
    unsafe fn destroy(self) {
        DEVICE.destroy_shader_module(self, ALLOC)
    }
}

impl RawHandle for vk::Framebuffer {}
impl Create<&vk::FramebufferCreateInfo> for vk::Framebuffer {
    unsafe fn create(info: &vk::FramebufferCreateInfo) -> VkResult<Self> {
        DEVICE.create_framebuffer(info, ALLOC)
    }
}
impl Destroy for vk::Framebuffer {
    unsafe fn destroy(self) {
        DEVICE.destroy_framebuffer(self, ALLOC)
    }
}

impl RawHandle for vk::Sampler {}
impl Create<&vk::SamplerCreateInfo> for vk::Sampler {
    unsafe fn create(info: &vk::SamplerCreateInfo) -> VkResult<Self> {
        DEVICE.create_sampler(info, ALLOC)
    }
}
impl Destroy for vk::Sampler {
    unsafe fn destroy(self) {
        DEVICE.destroy_sampler(self, ALLOC)
    }
}
