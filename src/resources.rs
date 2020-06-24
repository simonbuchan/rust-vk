use crate::device::{self, AsRawHandle};
use ash::{prelude::*, vk};

pub struct Mesh {
    pub memory: device::Memory,
    pub vertex_buffer: device::BufferObject,
    pub index_buffer: device::BufferObject,
    pub count: u32,
}

impl Mesh {
    pub fn create<Vertex: Copy>(vertices: &[Vertex], indices: &[u32]) -> VkResult<Self> {
        let vertex_buffer = device::BufferObject::create(
            device::size_of_val(vertices),
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer = device::BufferObject::create(
            device::size_of_val(indices),
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;

        // compute combined memory requirements
        let vertex_requirements = vertex_buffer.memory_requirements();
        let index_requirements = index_buffer.memory_requirements();
        let merged_size = vertex_requirements.size + index_requirements.size;

        let vertices_offset = 0;
        let indices_offset = vertex_requirements.size as usize;

        let memory = device::Memory::allocate_mappable(
            merged_size,
            device::MemoryTypeMask(
                vertex_requirements.memory_type_bits & index_requirements.memory_type_bits,
            ),
        )?;

        // write memory
        let mut mapping = memory.map(0, merged_size as usize)?;
        mapping.write_slice(vertices_offset, vertices);
        mapping.write_slice(indices_offset, indices);
        drop(mapping);

        // bind buffers to memory
        vertex_buffer.bind_memory(&memory, vertices_offset as vk::DeviceSize)?;
        index_buffer.bind_memory(&memory, indices_offset as vk::DeviceSize)?;

        Ok(Self {
            memory,
            vertex_buffer,
            index_buffer,
            count: indices.len() as u32,
        })
    }

    pub fn draw(&self, cmd: &device::CommandBufferRenderPassRecorder) {
        cmd.bind_vertex_buffer(0, self.vertex_buffer.as_raw());
        cmd.bind_index_buffer(self.index_buffer.as_raw());
        cmd.draw_indexed(self.count);
    }
}

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub image: device::Image,
    pub image_view: device::ImageView,
    pub sampler: device::Sampler,
}

impl Texture {
    pub fn create_2d(width: u32, height: u32) -> VkResult<Texture> {
        let image = device::Image::create_2d(
            (width, height),
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::SAMPLED,
            vk::ImageLayout::PREINITIALIZED,
            device::MemoryTypeMask::mappable(),
        )?;

        let image_view = device::ImageView::create_2d(
            image.object.as_raw(),
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageAspectFlags::COLOR,
        )?;

        let sampler = device::Sampler::nearest()?;

        Ok(Self {
            width,
            height,
            image,
            image_view,
            sampler,
        })
    }

    pub fn begin_upload(&self) -> VkResult<TextureUpload> {
        let layout = self.image.object.color_subresource_layout();
        let map = self.image.memory.map(layout.offset, layout.size as usize)?;
        Ok(TextureUpload {
            texture: self,
            layout,
            map,
        })
    }
}

pub type Texel = [u8; 4];

pub struct TextureUpload<'a> {
    texture: &'a Texture,
    layout: vk::SubresourceLayout,
    map: device::MemoryMapping,
}

impl<'a> TextureUpload<'a> {
    pub fn row(&mut self, y: u32) -> &mut [Texel] {
        let offset = (self.layout.offset + self.layout.row_pitch * y as vk::DeviceSize) as usize;
        let len = self.texture.width as usize;
        self.map.slice::<Texel>(offset, len)
    }

    pub fn upload_before(self, dst_stage_mask: vk::PipelineStageFlags) -> VkResult<()> {
        let texture = self.texture;
        drop(self);
        let recorder = device::CommandBuffer::create()?;
        recorder.image_memory_barrier(
            vk::PipelineStageFlags::HOST,
            dst_stage_mask,
            &vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::HOST_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::PREINITIALIZED)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(texture.image.object.as_raw())
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .level_count(1)
                        .layer_count(1)
                        .build(),
                ),
        );
        recorder.end()?.submit()?;
        Ok(())
    }
}
