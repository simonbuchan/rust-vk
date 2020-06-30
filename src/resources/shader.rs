use crate::device::Owned;
use crate::globals::*;

pub struct Compiler(shaderc::Compiler);

impl Compiler {
    pub fn new() -> Self {
        Self(shaderc::Compiler::new().unwrap())
    }

    pub fn compile_vertex(&mut self, source: &str) -> Result<Shader> {
        self.compile(
            shaderc::ShaderKind::Vertex,
            vk::ShaderStageFlags::VERTEX,
            source,
        )
    }

    pub fn compile_fragment(&mut self, source: &str) -> Result<Shader> {
        self.compile(
            shaderc::ShaderKind::Fragment,
            vk::ShaderStageFlags::FRAGMENT,
            source,
        )
    }

    pub fn compile(
        &mut self,
        compiler_type: shaderc::ShaderKind,
        vk_stage: vk::ShaderStageFlags,
        source: &str,
    ) -> Result<Shader> {
        let artifact = self
            .0
            .compile_into_spirv(source, compiler_type, "input", "main", None)?;
        Ok(Shader::new(artifact.as_binary(), vk_stage)?)
    }
}

pub struct Shader(Owned<vk::ShaderModule>, vk::ShaderStageFlags);

impl AsRef<vk::ShaderModule> for Shader {
    fn as_ref(&self) -> &vk::ShaderModule {
        self.0.as_ref()
    }
}

impl Shader {
    pub fn new(code: &[u32], stage: vk::ShaderStageFlags) -> VkResult<Self> {
        let owned =
            unsafe { Owned::create(&vk::ShaderModuleCreateInfo::builder().code(code).build())? };
        Ok(Self(owned, stage))
    }

    pub fn stage(&self) -> vk::ShaderStageFlags {
        self.1
    }
}
