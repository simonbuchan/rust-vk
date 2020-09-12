use crate::device;
use crate::device::{AsRawHandle, MemoryTypeMask};
use crate::globals::*;
use crate::resources;
use std::fs;
use std::io::{Read, Seek};

// http://github.khronos.org/KTX-Specification/

#[derive(Debug)]
#[repr(C)]
struct Header {
    pub signature: [u8; 12],
    pub format: vk::Format,
    pub type_size: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub layers: u32,
    pub faces: u32,
    pub levels: u32,
    pub supercompression_scheme: u32,
    pub dfd_byte_offset: u32,
    pub dfd_byte_length: u32,
    pub kvd_byte_offset: u32,
    pub kvd_byte_length: u32,
    pub sgd_byte_offset: u64,
    pub sgd_byte_length: u64,
}

#[derive(Debug)]
#[repr(C)]
struct LevelIndex {
    pub byte_offset: u64,
    pub byte_length: u64,
    pub uncompressed_byte_length: u64,
}

pub fn load_ktx(path: &str) -> Result<resources::Texture> {
    let mut file = fs::File::open(path)?;

    let mut header: Header = unsafe { std::mem::zeroed() };
    file.read_exact(unsafe { std::mem::transmute::<_, &mut [u8; 80]>(&mut header) })?;
    // println!("KTX header for {:?}: {:?}", path, header);

    if &header.signature != b"\xabKTX 20\xbb\r\n\x1a\n" {
        return Err(Error::Ktx);
    }

    let num_levels = header.levels.max(1) as usize;
    let mut level_indices = Vec::<LevelIndex>::with_capacity(num_levels);
    file.read_exact(unsafe {
        let start = level_indices.as_mut_ptr();
        let end = start.add(num_levels);
        std::slice::from_raw_parts_mut(start.cast(), end as usize - start as usize)
    })?;
    unsafe { level_indices.set_len(num_levels) };

    if header.depth != 0 {
        return Err(Error::Ktx);
    }
    if header.layers != 0 {
        return Err(Error::Ktx);
    }
    if header.supercompression_scheme != 0 {
        return Err(Error::Ktx);
    }

    let level_data_start = level_indices.iter().map(|i| i.byte_offset).min().unwrap();
    let level_data_end = level_indices
        .iter()
        .map(|i| i.byte_offset + i.byte_length)
        .max()
        .unwrap();
    let level_data_size = level_data_end - level_data_start;

    let image_object = match header.faces {
        1 => device::ImageObject::create_2d(
            (header.width, header.height),
            header.levels,
            header.format,
            vk::SampleCountFlags::TYPE_1,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        ),
        6 => device::ImageObject::create_cube(
            (header.width, header.height),
            header.levels,
            header.format,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        ),
        _ => return Err(Error::Ktx),
    }?;

    let data_buffer = device::Buffer::create(level_data_size, vk::BufferUsageFlags::TRANSFER_SRC)?;
    let mut data_mapping = data_buffer.memory.map(0, level_data_size as usize)?;
    file.seek(std::io::SeekFrom::Start(level_data_start))?;
    file.read_exact(data_mapping.slice(0, level_data_size as usize))?;
    drop(data_mapping);

    // Allocates and binds memory
    let image = device::Image::create(image_object, MemoryTypeMask::any())?;

    // Copy from level staging buffer to each image mip level
    let recording = device::CommandBuffer::create()?;
    // Transition to be a transfer target
    recording.image_transition(
        vk::PipelineStageFlags::HOST,
        vk::PipelineStageFlags::TRANSFER,
        &[vk::ImageMemoryBarrier::builder()
            .image(image.object.as_raw())
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS)
                    .level_count(vk::REMAINING_MIP_LEVELS)
                    .build(),
            )
            .build()],
    );

    for (level, indices) in level_indices.iter().enumerate() {
        // for layer in header.layers.max(1) {
        //     for face in header.faces {
        recording.copy_buffer_to_image(
            data_buffer.as_raw(),
            image.object.as_raw(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[vk::BufferImageCopy::builder()
                .buffer_offset(indices.byte_offset - level_data_start)
                .image_subresource(
                    vk::ImageSubresourceLayers::builder()
                        .layer_count(header.layers.max(1) * header.faces)
                        .mip_level(level as u32)
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .build(),
                )
                .image_extent(vk::Extent3D {
                    width: 1.max(header.width >> level),
                    height: 1.max(header.height >> level),
                    depth: 1.max(header.depth >> level),
                })
                .build()],
        );
        // }
        // }
    }
    recording.image_transition(
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        &[vk::ImageMemoryBarrier::builder()
            .image(image.object.as_raw())
            .src_access_mask(vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS)
                    .level_count(vk::REMAINING_MIP_LEVELS)
                    .build(),
            )
            .build()],
    );
    recording.end()?.submit()?;

    let image_view = device::ImageView::create(
        image.object.as_raw(),
        if header.faces == 6 {
            vk::ImageViewType::CUBE
        } else {
            vk::ImageViewType::TYPE_2D
        },
        header.format,
        vk::ImageAspectFlags::COLOR,
    )?;
    let sampler = device::Sampler::linear()?;

    let texture = resources::Texture {
        width: header.width,
        height: header.height,
        image,
        image_view,
        sampler,
    };

    Ok(texture)
}
