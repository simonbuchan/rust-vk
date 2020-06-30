use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Window(winit::error::OsError),
    VkEntry(ash::LoadingError),
    VkInstance(ash::InstanceError),
    VkSupport,
    Compiler(shaderc::Error),
    Vk(ash::vk::Result),
}

impl From<shaderc::Error> for Error {
    fn from(inner: shaderc::Error) -> Error {
        Self::Compiler(inner)
    }
}

impl From<ash::vk::Result> for Error {
    fn from(inner: ash::vk::Result) -> Error {
        Self::Vk(inner)
    }
}

impl From<ash::LoadingError> for Error {
    fn from(inner: ash::LoadingError) -> Self {
        Self::VkEntry(inner)
    }
}

impl From<ash::InstanceError> for Error {
    fn from(inner: ash::InstanceError) -> Self {
        Self::VkInstance(inner)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Window(inner) => fmt::Display::fmt(inner, f),
            Self::VkEntry(inner) => fmt::Display::fmt(inner, f),
            Self::VkInstance(inner) => fmt::Display::fmt(inner, f),
            Self::Vk(inner) => fmt::Display::fmt(inner, f),
            Self::VkSupport => f.write_str("Missing support"),
            Self::Compiler(inner) => fmt::Display::fmt(inner, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Window(inner) => Some(inner),
            Self::VkEntry(inner) => Some(inner),
            Self::VkInstance(inner) => Some(inner),
            Self::Vk(inner) => Some(inner),
            Self::VkSupport => None,
            Self::Compiler(inner) => Some(inner),
        }
    }
}
