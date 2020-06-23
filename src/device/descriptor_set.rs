use super::*;

impl RawHandle for vk::DescriptorPool {}
impl Create<&vk::DescriptorPoolCreateInfo> for vk::DescriptorPool {
    unsafe fn create(info: &vk::DescriptorPoolCreateInfo) -> VkResult<Self> {
        DEVICE.create_descriptor_pool(info, ALLOC)
    }
}
impl Destroy for vk::DescriptorPool {
    unsafe fn destroy(self) {
        DEVICE.destroy_descriptor_pool(self, ALLOC)
    }
}

pub struct DescriptorPool(Owned<vk::DescriptorPool>);

impl AsRef<vk::DescriptorPool> for DescriptorPool {
    fn as_ref(&self) -> &vk::DescriptorPool {
        self.0.as_ref()
    }
}

impl DescriptorPool {
    pub fn create(
        max_sets: u32,
        pool_sizes: &[vk::DescriptorPoolSize],
    ) -> VkResult<DescriptorPool> {
        unsafe {
            let owned = Owned::create(
                &vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(max_sets)
                    .pool_sizes(pool_sizes)
                    .build(),
            )?;
            Ok(Self(owned))
        }
    }

    pub fn allocate(&self, layout: vk::DescriptorSetLayout) -> VkResult<DescriptorSet> {
        unsafe {
            let sets = DEVICE.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(self.as_raw())
                    .set_layouts(&[layout])
                    .build(),
            )?;
            Ok(DescriptorSet(sets[0]))
        }
    }
}

impl RawHandle for vk::DescriptorSet {}

pub struct DescriptorSet(vk::DescriptorSet);

impl AsRef<vk::DescriptorSet> for DescriptorSet {
    fn as_ref(&self) -> &vk::DescriptorSet {
        &self.0
    }
}

impl DescriptorSet {
    pub fn update_buffer(
        &self,
        binding: u32,
        ty: vk::DescriptorType,
        buffer: vk::Buffer,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
    ) {
        unsafe {
            DEVICE.update_descriptor_sets(
                &[vk::WriteDescriptorSet::builder()
                    .dst_set(self.as_raw())
                    .dst_binding(binding)
                    .descriptor_type(ty)
                    .buffer_info(&[vk::DescriptorBufferInfo {
                        buffer,
                        offset,
                        range: size,
                    }])
                    .build()],
                &[],
            )
        }
    }

    pub fn update_image(&self, binding: u32, image_view: vk::ImageView, layout: vk::ImageLayout) {
        self.update_image_impl(
            binding,
            vk::DescriptorType::SAMPLER,
            vk::Sampler::null(),
            image_view,
            layout,
        )
    }

    pub fn update_sampler(&self, binding: u32, sampler: vk::Sampler) {
        self.update_image_impl(
            binding,
            vk::DescriptorType::SAMPLER,
            sampler,
            vk::ImageView::null(),
            vk::ImageLayout::UNDEFINED,
        )
    }

    pub fn update_combined_image_sampler(
        &self,
        binding: u32,
        sampler: vk::Sampler,
        image_view: vk::ImageView,
        image_layout: vk::ImageLayout,
    ) {
        self.update_image_impl(
            binding,
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            sampler,
            image_view,
            image_layout,
        )
    }

    fn update_image_impl(
        &self,
        binding: u32,
        ty: vk::DescriptorType,
        sampler: vk::Sampler,
        image_view: vk::ImageView,
        image_layout: vk::ImageLayout,
    ) {
        unsafe {
            DEVICE.update_descriptor_sets(
                &[vk::WriteDescriptorSet::builder()
                    .dst_set(self.as_raw())
                    .dst_binding(binding)
                    .descriptor_type(ty)
                    .image_info(&[vk::DescriptorImageInfo {
                        sampler,
                        image_view,
                        image_layout,
                    }])
                    .build()],
                &[],
            )
        }
    }
}
