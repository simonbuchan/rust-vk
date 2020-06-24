use super::*;

impl RawHandle for vk::PipelineLayout {}
impl Create<&vk::PipelineLayoutCreateInfo> for vk::PipelineLayout {
    unsafe fn create(info: &vk::PipelineLayoutCreateInfo) -> VkResult<Self> {
        DEVICE.create_pipeline_layout(info, ALLOC)
    }
}
impl Destroy for vk::PipelineLayout {
    unsafe fn destroy(self) {
        DEVICE.destroy_pipeline_layout(self, ALLOC)
    }
}
pub struct PipelineLayout(Owned<vk::PipelineLayout>);

impl AsRef<vk::PipelineLayout> for PipelineLayout {
    fn as_ref(&self) -> &vk::PipelineLayout {
        self.0.as_ref()
    }
}

impl PipelineLayout {
    pub fn create(set_layouts: &[vk::DescriptorSetLayout]) -> VkResult<Self> {
        let owned = unsafe {
            Owned::create(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(set_layouts)
                    .build(),
            )?
        };
        Ok(Self(owned))
    }
}

impl RawHandle for vk::PipelineCache {}
impl Create<&vk::PipelineCacheCreateInfo> for vk::PipelineCache {
    unsafe fn create(info: &vk::PipelineCacheCreateInfo) -> VkResult<Self> {
        DEVICE.create_pipeline_cache(info, ALLOC)
    }
}
impl Destroy for vk::PipelineCache {
    unsafe fn destroy(self) {
        DEVICE.destroy_pipeline_cache(self, ALLOC)
    }
}
pub struct PipelineCache(Owned<vk::PipelineCache>);

impl AsRef<vk::PipelineCache> for PipelineCache {
    fn as_ref(&self) -> &vk::PipelineCache {
        self.0.as_ref()
    }
}

impl PipelineCache {
    pub fn create() -> VkResult<Self> {
        let owned = unsafe { Owned::create(&vk::PipelineCacheCreateInfo::default()) }?;
        Ok(Self(owned))
    }

    pub fn create_pipeline(&self, infos: &vk::GraphicsPipelineCreateInfo) -> VkResult<Pipeline> {
        match unsafe {
            DEVICE.create_graphics_pipelines(self.as_raw(), std::slice::from_ref(infos), ALLOC)
        } {
            Ok(results) => Ok(Pipeline(unsafe { Owned::from_raw(results[0]) })),
            Err((_results, err)) => Err(err),
        }
    }
}

impl RawHandle for vk::Pipeline {}
impl Destroy for vk::Pipeline {
    unsafe fn destroy(self) {
        DEVICE.destroy_pipeline(self, ALLOC)
    }
}
pub struct Pipeline(Owned<vk::Pipeline>);

impl AsRef<vk::Pipeline> for Pipeline {
    fn as_ref(&self) -> &vk::Pipeline {
        self.0.as_ref()
    }
}
