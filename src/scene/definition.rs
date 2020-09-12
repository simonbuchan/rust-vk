use ash::vk;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Scene {
    #[serde(default)]
    pub programs: Vec<Program>,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(default)]
    pub textures: Vec<TextureFile>,
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
pub struct TextureFile {
    pub id: u32,
    pub format: TextureFormat,
    #[serde(default)]
    pub space: TextureColorSpace,
    pub path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextureFormat {
    Png,
    Ktx,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextureColorSpace {
    Linear,
    Srgb,
}

impl Default for TextureColorSpace {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Deserialize)]
pub struct File {
    pub id: u32,
    pub path: String,
}

#[derive(Deserialize)]
pub struct Mesh {
    // pub id: u32,
    // pub name: String,
    #[serde(default)]
    pub transform: Transform,
    pub material: u32,
    #[serde(default)]
    pub bindings: Vec<MeshBinding>,
    pub indices: MeshIndices,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Transform {
    Literal([[f32; 4]; 4]),
    TRS {
        #[serde(default)]
        translation: Option<[f32; 3]>,
        #[serde(default)]
        rotation: Option<[f32; 4]>,
        #[serde(default)]
        scale: Option<[f32; 3]>,
    },
    Identity,
}

impl Default for Transform {
    fn default() -> Self {
        Self::Identity
    }
}

impl From<&Transform> for crate::math::Mat4 {
    fn from(value: &Transform) -> Self {
        match value {
            Transform::Literal([x, y, z, w]) => {
                Self::from([(*x).into(), (*y).into(), (*z).into(), (*w).into()])
            }
            Transform::TRS {
                translation,
                rotation,
                scale,
            } => {
                let mut value = Self::IDENTITY;
                if let Some(&t) = translation.as_ref() {
                    value = value * Self::translate(t.into());
                }
                if let Some(&r) = rotation.as_ref() {
                    value = value * Self::rotate(r.into());
                }
                if let Some(&s) = scale.as_ref() {
                    value = value * Self::scale(s.into());
                }
                value
            }
            Transform::Identity => Self::IDENTITY,
        }
    }
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

#[derive(Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeshIndexFormat {
    U16,
    U32,
}

impl From<MeshIndexFormat> for vk::IndexType {
    fn from(value: MeshIndexFormat) -> Self {
        match value {
            MeshIndexFormat::U16 => Self::UINT16,
            MeshIndexFormat::U32 => Self::UINT32,
        }
    }
}

#[derive(Deserialize)]
pub struct BufferView {
    pub buffer: u32,
    pub offset: u64,
    pub size: u64,
}
