use super::*;

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
}

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
pub struct PipelineLayout(vk::PipelineLayout);

impl RawHandle for vk::Pipeline {}
impl CreateVec<(vk::PipelineCache, &[vk::GraphicsPipelineCreateInfo])> for vk::Pipeline {
    unsafe fn create_vec(
        info: (vk::PipelineCache, &[vk::GraphicsPipelineCreateInfo]),
    ) -> VkResult<Vec<Self>> {
        match DEVICE.create_graphics_pipelines(info.0, info.1, ALLOC) {
            Ok(result) => Ok(result),
            Err((pipelines, err)) => {
                for p in pipelines {
                    if p != vk::Pipeline::null() {
                        p.destroy()
                    }
                }
                Err(err)
            }
        }
    }
}
impl Create<&vk::GraphicsPipelineCreateInfo> for vk::Pipeline {
    unsafe fn create(info: &vk::GraphicsPipelineCreateInfo) -> VkResult<Self> {
        Self::create((vk::PipelineCache::null(), info))
    }
}
impl Create<(vk::PipelineCache, &vk::GraphicsPipelineCreateInfo)> for vk::Pipeline {
    unsafe fn create(info: (vk::PipelineCache, &vk::GraphicsPipelineCreateInfo)) -> VkResult<Self> {
        Ok(Self::create_vec((info.0, std::slice::from_ref(info.1)))?[0])
    }
}
impl Destroy for vk::Pipeline {
    unsafe fn destroy(self) {
        DEVICE.destroy_pipeline(self, ALLOC)
    }
}
pub struct Pipeline(vk::Pipeline);
