use super::*;

impl RawHandle for vk::CommandBuffer {}

pub struct CommandBuffer(vk::CommandBuffer);

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe { DEVICE.free_command_buffers(GRAPHICS_COMMAND_POOL, &[self.0]) };
    }
}

impl AsRef<vk::CommandBuffer> for CommandBuffer {
    fn as_ref(&self) -> &vk::CommandBuffer {
        &self.0
    }
}

impl CommandBuffer {
    pub fn create() -> VkResult<CommandBufferRecorder> {
        unsafe {
            let command_buffers = DEVICE.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(GRAPHICS_COMMAND_POOL)
                    .command_buffer_count(1)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .build(),
            )?;
            let result = Self(command_buffers[0]);
            DEVICE.begin_command_buffer(result.as_raw(), &vk::CommandBufferBeginInfo::default())?;
            Ok(CommandBufferRecorder(result))
        }
    }

    pub fn submit(self) -> VkResult<()> {
        unsafe {
            let submit_fence = Fence::create()?;

            DEVICE.queue_submit(
                GRAPHICS_QUEUE,
                &[vk::SubmitInfo::builder()
                    .command_buffers(&[self.as_raw()])
                    .build()],
                submit_fence.as_raw(),
            )?;

            submit_fence.wait()?;

            Ok(())
        }
    }

    pub fn submit_after(
        self,
        wait_semaphore: vk::Semaphore,
        wait_dst_stage_mask: vk::PipelineStageFlags,
    ) -> VkResult<()> {
        unsafe {
            let submit_fence = Fence::create()?;

            DEVICE.queue_submit(
                GRAPHICS_QUEUE,
                &[vk::SubmitInfo::builder()
                    .command_buffers(&[self.as_raw()])
                    .wait_semaphores(&[wait_semaphore])
                    .wait_dst_stage_mask(&[wait_dst_stage_mask])
                    .build()],
                submit_fence.as_raw(),
            )?;

            submit_fence.wait()?;

            Ok(())
        }
    }
}

pub struct CommandBufferRecorder(CommandBuffer);

impl AsRef<vk::CommandBuffer> for CommandBufferRecorder {
    fn as_ref(&self) -> &vk::CommandBuffer {
        self.0.as_ref()
    }
}

impl CommandBufferRecorder {
    pub fn end(self) -> VkResult<CommandBuffer> {
        unsafe { DEVICE.end_command_buffer(self.as_raw()) }?;
        Ok(self.0)
    }

    pub fn begin_render_pass(
        self,
        info: &vk::RenderPassBeginInfo,
    ) -> CommandBufferRenderPassRecorder {
        unsafe {
            DEVICE.cmd_begin_render_pass(self.as_raw(), info, vk::SubpassContents::INLINE);
        }
        CommandBufferRenderPassRecorder(self)
    }

    pub fn image_memory_barrier(
        &self,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        barrier: &vk::ImageMemoryBarrier,
    ) {
        unsafe {
            DEVICE.cmd_pipeline_barrier(
                self.as_raw(),
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                std::slice::from_ref(barrier),
            )
        }
    }
}

pub struct CommandBufferRenderPassRecorder(CommandBufferRecorder);

impl AsRef<vk::CommandBuffer> for CommandBufferRenderPassRecorder {
    fn as_ref(&self) -> &vk::CommandBuffer {
        self.0.as_ref()
    }
}

impl CommandBufferRenderPassRecorder {
    pub fn end_render_pass(self) -> CommandBufferRecorder {
        unsafe { DEVICE.cmd_end_render_pass(self.as_raw()) };
        self.0
    }

    pub fn bind_pipeline(&self, pipeline: vk::Pipeline) {
        unsafe {
            DEVICE.cmd_bind_pipeline(self.as_raw(), vk::PipelineBindPoint::GRAPHICS, pipeline);
        }
    }

    pub fn bind_descriptor_set(
        &self,
        pipeline_layout: vk::PipelineLayout,
        descriptor_set: vk::DescriptorSet,
    ) {
        unsafe {
            DEVICE.cmd_bind_descriptor_sets(
                self.as_raw(),
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
        }
    }

    pub fn bind_vertex_buffer(&self, binding: u32, buffer: vk::Buffer) {
        unsafe { DEVICE.cmd_bind_vertex_buffers(self.as_raw(), binding, &[buffer], &[0]) };
    }

    pub fn bind_index_buffer(&self, buffer: vk::Buffer) {
        unsafe { DEVICE.cmd_bind_index_buffer(self.as_raw(), buffer, 0, vk::IndexType::UINT32) };
    }

    pub fn draw(&self, vertex_count: u32) {
        unsafe { DEVICE.cmd_draw(self.as_raw(), vertex_count, 1, 0, 0) };
    }

    pub fn draw_indexed(&self, index_count: u32) {
        unsafe { DEVICE.cmd_draw_indexed(self.as_raw(), index_count, 1, 0, 0, 0) };
    }
}
