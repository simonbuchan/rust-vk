use ash::prelude::VkResult;
use ash::vk;

use crate::device::{self, AsRawHandle};
use crate::error::*;
use crate::math::Mat4;
use crate::resources;

use super::definition;

pub struct MaterialProgram {
    pub cache: device::PipelineCache,
    pub descriptors_layout: device::DescriptorSetLayout,
    pub pipeline_layout: device::PipelineLayout,
    pub vs: resources::Shader,
    pub fs: resources::Shader,
    pub vertex_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
}

impl MaterialProgram {
    pub fn create(
        definition: &definition::Program,
        compiler: &mut resources::Compiler,
        view_descriptors_layout: vk::DescriptorSetLayout,
    ) -> Result<Self> {
        let cache = device::PipelineCache::create()?;

        let mut layout_builder = device::DescriptorSetLayout::builder();
        for descriptor in &definition.descriptors {
            layout_builder = layout_builder.add_basic(
                descriptor.binding,
                descriptor.ty.into(),
                descriptor
                    .stages
                    .iter()
                    .fold(vk::ShaderStageFlags::empty(), |a, &s| a | s.into()),
            );
        }

        let descriptors_layout = layout_builder.build()?;

        let pipeline_layout = device::PipelineLayout::create(
            &[view_descriptors_layout, descriptors_layout.as_raw()],
            &[vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::VERTEX,
                offset: 0,
                size: std::mem::size_of::<Mat4>() as u32,
            }],
        )?;

        let vs = compiler.compile_vertex(&definition.vertex)?;
        let fs = compiler.compile_fragment(&definition.fragment)?;

        let vertex_binding_descriptions = definition
            .vertex_input
            .iter()
            .map(|b| {
                vk::VertexInputBindingDescription::builder()
                    .binding(b.binding)
                    .stride(b.stride)
                    .input_rate(vk::VertexInputRate::VERTEX)
                    .build()
            })
            .collect::<Vec<_>>();
        let vertex_attribute_descriptions = definition
            .vertex_input
            .iter()
            .flat_map(|b| {
                b.attributes
                    .iter()
                    .map(|a| {
                        vk::VertexInputAttributeDescription::builder()
                            .binding(b.binding)
                            .location(a.location)
                            .offset(a.offset)
                            .format(a.format.into())
                            .build()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(Self {
            cache,
            descriptors_layout,
            pipeline_layout,
            vs,
            fs,
            vertex_binding_descriptions,
            vertex_attribute_descriptions,
        })
    }

    pub fn create_material_pipeline(
        &self,
        render_pass: vk::RenderPass,
        samples: vk::SampleCountFlags,
    ) -> VkResult<device::Pipeline> {
        let name = std::ffi::CString::new("main").unwrap();
        self.cache.create_pipeline(
            &vk::GraphicsPipelineCreateInfo::builder()
                .stages(&[
                    vk::PipelineShaderStageCreateInfo::builder()
                        .name(&name)
                        .module(self.vs.as_raw())
                        .stage(vk::ShaderStageFlags::VERTEX)
                        .build(),
                    vk::PipelineShaderStageCreateInfo::builder()
                        .name(&name)
                        .module(self.fs.as_raw())
                        .stage(vk::ShaderStageFlags::FRAGMENT)
                        .build(),
                ])
                .vertex_input_state(
                    &vk::PipelineVertexInputStateCreateInfo::builder()
                        .vertex_binding_descriptions(&self.vertex_binding_descriptions)
                        .vertex_attribute_descriptions(&self.vertex_attribute_descriptions),
                )
                .input_assembly_state(
                    &vk::PipelineInputAssemblyStateCreateInfo::builder()
                        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                        .build(),
                )
                .viewport_state(
                    &vk::PipelineViewportStateCreateInfo::builder()
                        .viewport_count(1)
                        .scissor_count(1)
                        .build(),
                )
                .rasterization_state(
                    &vk::PipelineRasterizationStateCreateInfo::builder()
                        .line_width(1.0)
                        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                        .cull_mode(vk::CullModeFlags::BACK)
                        .build(),
                )
                .multisample_state(
                    &vk::PipelineMultisampleStateCreateInfo::builder()
                        .rasterization_samples(samples)
                        .build(),
                )
                .color_blend_state(
                    &vk::PipelineColorBlendStateCreateInfo::builder()
                        .attachments(&[vk::PipelineColorBlendAttachmentState::builder()
                            .color_write_mask(vk::ColorComponentFlags::all())
                            .blend_enable(true)
                            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                            .src_color_blend_factor(vk::BlendFactor::SRC_COLOR)
                            .color_blend_op(vk::BlendOp::ADD)
                            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                            .src_alpha_blend_factor(vk::BlendFactor::ONE)
                            .alpha_blend_op(vk::BlendOp::ADD)
                            .build()])
                        .build(),
                )
                .dynamic_state(
                    &vk::PipelineDynamicStateCreateInfo::builder()
                        .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]),
                )
                .layout(self.pipeline_layout.as_raw())
                .render_pass(render_pass)
                .subpass(0)
                .build(),
        )
    }
}

pub struct Material {
    pub program: u32,
    pub pipeline: device::Pipeline,
    pub descriptors: device::DescriptorSet,
}
