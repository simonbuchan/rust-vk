use ash::vk;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Scene {
    #[serde(default)]
    pub programs: Vec<Program>,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(default)]
    pub textures: Vec<File>,
    #[serde(default)]
    pub buffers: Vec<File>,
    #[serde(default)]
    pub meshes: Vec<Mesh>,
}

#[derive(Deserialize)]
pub struct Program {
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
pub struct Material {
    pub id: u32,
    pub program: u32,
    pub textures: Vec<MaterialTexture>,
}

#[derive(Deserialize)]
pub struct MaterialTexture {
    pub location: u32,
    pub texture: u32,
}

#[derive(Deserialize)]
pub struct File {
    pub id: u32,
    pub path: String,
}

#[derive(Deserialize)]
pub struct Mesh {
    // pub name: String,
    pub material: u32,
    #[serde(default)]
    pub bindings: Vec<MeshBinding>,
    pub indices: MeshIndices,
}

#[derive(Deserialize)]
pub struct MeshBinding {
    pub binding: u32,
    pub view: BufferView,
}

#[derive(Deserialize)]
pub struct MeshIndices {
    pub count: u32,
    pub format: MeshIndexFormat,
    pub view: BufferView,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeshIndexFormat {
    U16,
    U32,
}

#[derive(Deserialize)]
pub struct BufferView {
    pub buffer: u32,
    pub offset: u32,
    pub size: u32,
}
