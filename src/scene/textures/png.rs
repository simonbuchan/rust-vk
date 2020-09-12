use crate::device;
use crate::device::AsRawHandle;
use crate::globals::*;
use crate::resources;
use std::fs;

pub fn load_png(path: &str, srgb: bool) -> Result<resources::Texture> {
    let (info, mut reader) = png::Decoder::new(std::fs::File::open(&path)?).read_info()?;

    let texture_buffer = match info.color_type {
        // https://github.com/image-rs/image-png/issues/239
        png::ColorType::RGB => expand_rgb(&info, &mut reader)?,
        png::ColorType::RGBA => {
            let texture_buffer_size = reader.output_buffer_size();
            let texture_buffer = device::Buffer::create(
                texture_buffer_size as vk::DeviceSize,
                vk::BufferUsageFlags::TRANSFER_SRC,
            )?;
            let mut mapping = texture_buffer.memory.map(0, texture_buffer_size)?;
            reader.next_frame(mapping.slice(0, texture_buffer_size))?;
            texture_buffer
        }
        _ => unimplemented!("png::ColorType::{:?}", info.color_type),
    };

    let texture = resources::Texture::create(
        info.width,
        info.height,
        if srgb {
            vk::Format::R8G8B8A8_SRGB
        } else {
            vk::Format::R8G8B8A8_UNORM
        },
    )?;
    texture.copy_from(texture_buffer.as_raw(), 0)?;

    Ok(texture)
}

fn expand_rgb(
    info: &png::OutputInfo,
    reader: &mut png::Reader<fs::File>,
) -> Result<device::Buffer> {
    let texture_buffer_size = (info.width * info.height * 4) as usize;
    let texture_buffer = device::Buffer::create(
        texture_buffer_size as vk::DeviceSize,
        vk::BufferUsageFlags::TRANSFER_SRC,
    )?;
    let mut mapping = texture_buffer.memory.map(0, texture_buffer_size)?;
    println!(
        "expanding RGB -> RGBA: {}x{} = {:#x} bytes",
        info.width, info.height, texture_buffer_size
    );
    let mut output: *mut u8 = mapping.slice(0, texture_buffer_size).as_mut_ptr();
    while let Some(row) = reader.next_row()? {
        let mut input = row.as_ptr();
        for _ in 0..info.width {
            unsafe {
                output.write(input.read());
                input = input.offset(1);
                output = output.offset(1);
                output.write(input.read());
                input = input.offset(1);
                output = output.offset(1);
                output.write(input.read());
                input = input.offset(1);
                output = output.offset(1);
                output.write(0xffu8);
                output = output.offset(1);
            }
        }
    }
    Ok(texture_buffer)
}
