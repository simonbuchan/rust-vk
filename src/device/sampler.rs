use super::*;

pub struct Sampler(Owned<vk::Sampler>);

impl AsRef<vk::Sampler> for Sampler {
    fn as_ref(&self) -> &vk::Sampler {
        self.0.as_ref()
    }
}

impl Sampler {
    pub fn create(info: &vk::SamplerCreateInfo) -> VkResult<Self> {
        let owned = unsafe { Owned::create(info) }?;
        Ok(Self(owned))
    }

    pub fn nearest() -> VkResult<Self> {
        Self::create(&vk::SamplerCreateInfo::builder())
    }

    pub fn linear() -> VkResult<Self> {
        Self::create(
            &vk::SamplerCreateInfo::builder()
                .min_filter(vk::Filter::LINEAR)
                .mag_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .min_lod(0.0)
                .max_lod(16.0)
                .anisotropy_enable(true)
                .max_anisotropy(16.0),
        )
    }
}
