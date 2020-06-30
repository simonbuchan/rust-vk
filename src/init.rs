use crate::device::{AsRawHandle, Owned};
use crate::globals::*;
use std::ffi::CStr;
use winit::platform::windows::WindowExtWindows;

pub fn init(window: &winit::window::Window) -> Result<Owned<vk::SurfaceKHR>> {
    unsafe {
        init_instance()?;

        let surface = Owned::create(
            &vk::Win32SurfaceCreateInfoKHR::builder()
                .hinstance(window.hinstance())
                .hwnd(window.hwnd())
                .build(),
        )?;

        if !select_physical_device_and_graphics_queue(surface.as_raw())? {
            return Err(Error::VkSupport);
        }

        let physical_device_props = INSTANCE.get_physical_device_properties(PHYSICAL_DEVICE);
        println!(
            "device: {:#x?} {:?}",
            physical_device_props.device_type,
            CStr::from_ptr(physical_device_props.device_name.as_ptr().cast()),
        );

        // Call required to initialize the surface capabilities when initializing swapchain.
        let _surface_capabilities = ext::SURFACE
            .get_physical_device_surface_capabilities(PHYSICAL_DEVICE, surface.as_raw())?;

        init_device()?;

        Ok(surface)
    }
}
