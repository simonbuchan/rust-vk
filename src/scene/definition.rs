use ash::vk;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Scene {
    pub materials: Vec<Material>,
    #[serde(default)]
    pub textures: Vec<File>,
    #[serde(default)]
    pub buffers: Vec<File>,
}

#[derive(Deserialize)]
pub struct Material {
    pub id: u32,
    pub vertex_input: Vec<VertexInputBinding>,
    #[serde(default)]
    pub descriptors: Vec<Descriptor>,
    pub vertex: String,
    pub fragment: String,
}

#[derive(Deserialize)]
pub struct VertexInputBinding {
    pub binding: u32,
    pub stride: u32,
    pub attributes: Vec<VertexAttribute>,
}

#[derive(Deserialize)]
pub struct VertexAttribute {
    pub location: u32,
    pub offset: u32,
    pub format: AttributeFormat,
}

#[derive(Copy, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttributeFormat {
    F32,
    Vec2,
    Vec3,
    Vec4,
    U32,
    UVec2,
    UVec3,
    UVec4,
}

impl Into<ash::vk::Format> for AttributeFormat {
    fn into(self) -> ash::vk::Format {
        use ash::vk::Format;
        match self {
            Self::F32 => Format::R32_SFLOAT,
            Self::Vec2 => Format::R32G32_SFLOAT,
            Self::Vec3 => Format::R32G32B32_SFLOAT,
            Self::Vec4 => Format::R32G32B32A32_SFLOAT,
            Self::U32 => Format::R32_UINT,
            Self::UVec2 => Format::R32G32_UINT,
            Self::UVec3 => Format::R32G32B32_UINT,
            Self::UVec4 => Format::R32G32B32A32_UINT,
        }
    }
}

#[derive(Deserialize)]
pub struct Descriptor {
    pub binding: u32,
    #[serde(rename = "type")]
    pub ty: DescriptorType,
    pub stages: Vec<StageType>,
}

#[derive(Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DescriptorType {
    Uniform,
    Texture,
}

impl Into<vk::DescriptorType> for DescriptorType {
    fn into(self) -> vk::DescriptorType {
        match self {
            Self::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
            Self::Texture => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        }
    }
}

#[derive(Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StageType {
    Vertex,
    Fragment,
}

impl Into<vk::ShaderStageFlags> for StageType {
    fn into(self) -> vk::ShaderStageFlags {
        match self {
            Self::Vertex => vk::ShaderStageFlags::VERTEX,
            Self::Fragment => vk::ShaderStageFlags::FRAGMENT,
        }
    }
}

#[derive(Deserialize)]
pub struct File {
    pub id: u32,
    pub path: String,
}
