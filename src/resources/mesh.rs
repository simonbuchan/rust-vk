use crate::device::{self, AsRawHandle};
use ash::{prelude::*, vk};

pub struct MeshObject {
    pub vertex_buffers: Vec<device::BufferObject>,
    pub index_buffer: device::BufferObject,
    pub count: u32,
}

impl MeshObject {
    pub fn draw(&self, cmd: &device::CommandBufferRenderPassRecorder) {
        for (i, buffer) in self.vertex_buffers.iter().enumerate() {
            cmd.bind_vertex_buffer(i as u32, buffer.as_raw());
        }
        cmd.bind_index_buffer(self.index_buffer.as_raw());
        cmd.draw_indexed(self.count);
    }
}

pub struct Mesh {
    pub memory: device::Memory,
    pub object: MeshObject,
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
            object: MeshObject {
                vertex_buffers: vec![vertex_buffer],
                index_buffer,
                count: indices.len() as u32,
            },
        })
    }

    pub fn draw(&self, cmd: &device::CommandBufferRenderPassRecorder) {
        self.object.draw(cmd);
    }
}
