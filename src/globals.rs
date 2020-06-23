use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ops::Deref;

pub use ash::prelude::*;
pub use ash::version::*;
pub use ash::vk;

use crate::globals::ext::DEBUG_UTILS;
use crate::init::InitResult;

pub struct AssumeInit<T>(MaybeUninit<T>);

impl<T> AssumeInit<T> {
    pub const fn new() -> Self {
        Self(MaybeUninit::uninit())
    }

    pub unsafe fn init(target: &mut Self, value: T) {
        target.0 = MaybeUninit::new(value);
    }
}

impl<T> Deref for AssumeInit<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.0.as_ptr() }
    }
}

pub static mut ENTRY: AssumeInit<ash::Entry> = AssumeInit::new();
pub static mut INSTANCE: AssumeInit<ash::Instance> = AssumeInit::new();
pub static mut PHYSICAL_DEVICE: vk::PhysicalDevice = vk::PhysicalDevice::null();
pub static mut MEMORY_PROPERTIES: AssumeInit<vk::PhysicalDeviceMemoryProperties> =
    AssumeInit::new();
pub static mut GRAPHICS_QUEUE_FAMILY_INDEX: u32 = u32::max_value();
pub static mut DEVICE: AssumeInit<ash::Device> = AssumeInit::new();
pub static mut GRAPHICS_QUEUE: vk::Queue = vk::Queue::null();
pub static mut GRAPHICS_COMMAND_POOL: vk::CommandPool = vk::CommandPool::null();
pub static mut ALLOC: Option<&ash::vk::AllocationCallbacks> = None;

pub mod ext {
    pub use ash::extensions::ext::DebugUtils;
    pub use ash::extensions::khr::{Surface, Swapchain, Win32Surface};

    use super::AssumeInit;

    pub static mut DEBUG_UTILS: AssumeInit<DebugUtils> = AssumeInit::new();
    pub static mut SURFACE: AssumeInit<Surface> = AssumeInit::new();
    pub static mut WIN32_SURFACE: AssumeInit<Win32Surface> = AssumeInit::new();
    pub static mut SWAPCHAIN: AssumeInit<Swapchain> = AssumeInit::new();
}

macro_rules! names {
    ($($ident: ident),* $(,)?) => {
        &[
            $( concat!(stringify!($ident), "\0").as_ptr().cast::<std::os::raw::c_char>() ,)*
        ]
    };
}

pub unsafe fn init_instance() -> InitResult<()> {
    AssumeInit::init(&mut ENTRY, ash::Entry::new()?);
    AssumeInit::init(
        &mut INSTANCE,
        ENTRY.create_instance(
            &vk::InstanceCreateInfo::builder()
                .enabled_layer_names(names![
                    VK_LAYER_KHRONOS_validation,
                    VK_LAYER_LUNARG_standard_validation,
                ])
                .enabled_extension_names(names![
                    VK_KHR_surface,
                    VK_KHR_win32_surface,
                    VK_EXT_debug_utils,
                ])
                .build(),
            ALLOC,
        )?,
    );
    // need to explicitly type these as Deref doesn't play well with generics?
    let entry: &ash::Entry = &ENTRY;
    let instance: &ash::Instance = &INSTANCE;

    AssumeInit::init(&mut ext::DEBUG_UTILS, ext::DebugUtils::new(entry, instance));
    DEBUG_UTILS.create_debug_utils_messenger(
        &vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(debug_callback))
            .build(),
        ALLOC,
    )?;

    AssumeInit::init(&mut ext::SURFACE, ext::Surface::new(entry, instance));
    AssumeInit::init(
        &mut ext::WIN32_SURFACE,
        ext::Win32Surface::new(entry, instance),
    );

    Ok(())
}

pub unsafe fn select_physical_device_and_graphics_queue(
    surface: vk::SurfaceKHR,
) -> InitResult<bool> {
    for pd in INSTANCE.enumerate_physical_devices()? {
        let qfps = INSTANCE.get_physical_device_queue_family_properties(pd);
        for (index, queue_family_props) in qfps.into_iter().enumerate() {
            if !queue_family_props
                .queue_flags
                .contains(vk::QueueFlags::GRAPHICS)
            {
                continue;
            }
            if !ext::SURFACE.get_physical_device_surface_support(pd, index as u32, surface)? {
                continue;
            }
            PHYSICAL_DEVICE = pd;
            GRAPHICS_QUEUE_FAMILY_INDEX = index as u32;
            AssumeInit::init(
                &mut MEMORY_PROPERTIES,
                INSTANCE.get_physical_device_memory_properties(PHYSICAL_DEVICE),
            );

            return Ok(true);
        }
    }

    Ok(false)
}

pub unsafe fn init_device() -> InitResult<()> {
    AssumeInit::init(&mut DEVICE, {
        let info = vk::DeviceCreateInfo::builder()
            .enabled_extension_names(names![VK_KHR_swapchain])
            .queue_create_infos(&[vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(GRAPHICS_QUEUE_FAMILY_INDEX)
                .queue_priorities(&[1.0])
                .build()])
            .build();

        INSTANCE.create_device(PHYSICAL_DEVICE, &info, ALLOC)?
    });
    let instance: &ash::Instance = &INSTANCE;
    let device: &ash::Device = &DEVICE;
    AssumeInit::init(&mut ext::SWAPCHAIN, ext::Swapchain::new(instance, device));

    GRAPHICS_QUEUE = DEVICE.get_device_queue(GRAPHICS_QUEUE_FAMILY_INDEX, 0);

    GRAPHICS_COMMAND_POOL = DEVICE.create_command_pool(
        &vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(GRAPHICS_QUEUE_FAMILY_INDEX)
            .build(),
        ALLOC,
    )?;

    Ok(())
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let data = &*p_callback_data;
    let message = CStr::from_ptr(data.p_message).to_string_lossy();
    if message.starts_with("Device Extension: ") {
        return vk::FALSE;
    }

    eprintln!(
        "{:?} {:?}: [{}: {}] {}",
        message_severity,
        message_types,
        CStr::from_ptr(data.p_message_id_name).to_string_lossy(),
        data.message_id_number,
        message,
    );

    if message_severity.intersects(
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
    ) {
        extern "system" {
            fn DebugBreak();
        }
        DebugBreak();
        std::process::exit(1);
    }

    vk::FALSE
}
