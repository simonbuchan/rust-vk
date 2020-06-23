use std::error::Error;
use std::fmt;

use crate::device::{AsRawHandle, Owned};
use crate::globals::*;
use std::ffi::CStr;
use winit::platform::windows::WindowExtWindows;

pub fn init(window: &winit::window::Window) -> InitResult<Owned<vk::SurfaceKHR>> {
    unsafe {
        init_instance()?;

        let surface = Owned::create(
            &vk::Win32SurfaceCreateInfoKHR::builder()
                .hinstance(window.hinstance())
                .hwnd(window.hwnd())
                .build(),
        )?;

        if !select_physical_device_and_graphics_queue(surface.as_raw())? {
            return Err(InitError::Support);
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

pub type InitResult<T> = Result<T, InitError>;

#[derive(Debug)]
pub enum InitError {
    Entry(ash::LoadingError),
    Instance(ash::InstanceError),
    Vk(vk::Result),
    Support,
}

impl From<ash::LoadingError> for InitError {
    fn from(inner: ash::LoadingError) -> Self {
        Self::Entry(inner)
    }
}

impl From<ash::InstanceError> for InitError {
    fn from(inner: ash::InstanceError) -> Self {
        Self::Instance(inner)
    }
}

impl From<vk::Result> for InitError {
    fn from(inner: vk::Result) -> Self {
        Self::Vk(inner)
    }
}

impl Error for InitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            InitError::Entry(inner) => Some(inner),
            InitError::Instance(inner) => Some(inner),
            InitError::Vk(inner) => Some(inner),
            InitError::Support => None,
        }
    }
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        match self {
            InitError::Entry(inner) => fmt::Display::fmt(inner, f),
            InitError::Instance(inner) => fmt::Display::fmt(inner, f),
            InitError::Vk(inner) => fmt::Display::fmt(inner, f),
            InitError::Support => f.write_str("Missing support"),
        }
    }
}
