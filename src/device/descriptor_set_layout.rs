use super::*;
use std::marker::PhantomData;

impl RawHandle for vk::DescriptorSetLayout {}
impl Create<&vk::DescriptorSetLayoutCreateInfo> for vk::DescriptorSetLayout {
    unsafe fn create(info: &vk::DescriptorSetLayoutCreateInfo) -> VkResult<Self> {
        DEVICE.create_descriptor_set_layout(info, ALLOC)
    }
}
impl Destroy for vk::DescriptorSetLayout {
    unsafe fn destroy(self) {
        DEVICE.destroy_descriptor_set_layout(self, ALLOC)
    }
}

pub struct DescriptorSetLayoutBuilder<'a> {
    bindings: Vec<vk::DescriptorSetLayoutBinding>,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> DescriptorSetLayoutBuilder<'a> {
    pub fn new() -> Self {
        Self {
            bindings: vec![],
            lifetime: PhantomData,
        }
    }

    pub fn build(self) -> VkResult<DescriptorSetLayout> {
        DescriptorSetLayout::create(&self.bindings)
    }

    pub fn add_uniform_buffer(self, binding: u32, stages: vk::ShaderStageFlags) -> Self {
        self.add_basic(binding, vk::DescriptorType::UNIFORM_BUFFER, stages)
    }

    pub fn add_combined_image_sampler(self, binding: u32, stages: vk::ShaderStageFlags) -> Self {
        self.add_basic(binding, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, stages)
    }

    fn add_basic(
        self,
        binding: u32,
        descriptor_type: vk::DescriptorType,
        stage_flags: vk::ShaderStageFlags,
    ) -> Self {
        self.add(
            vk::DescriptorSetLayoutBinding::builder()
                .binding(binding)
                .descriptor_count(1)
                .descriptor_type(descriptor_type)
                .stage_flags(stage_flags)
                .build(),
        )
    }

    fn add(mut self, binding: vk::DescriptorSetLayoutBinding) -> Self {
        self.bindings.push(binding);
        self
    }
}

pub struct DescriptorSetLayout(Owned<vk::DescriptorSetLayout>);

impl AsRef<vk::DescriptorSetLayout> for DescriptorSetLayout {
    fn as_ref(&self) -> &vk::DescriptorSetLayout {
        self.0.as_ref()
    }
}

impl DescriptorSetLayout {
    pub fn builder<'a>() -> DescriptorSetLayoutBuilder<'a> {
        DescriptorSetLayoutBuilder::new()
    }

    pub fn create(bindings: &[vk::DescriptorSetLayoutBinding]) -> VkResult<Self> {
        unsafe {
            let owned = Owned::create(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .bindings(bindings)
                    .build(),
            )?;
            Ok(Self(owned))
        }
    }
}
