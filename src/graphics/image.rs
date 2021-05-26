use crate::GraphicsDevice;
use wgpu::util::DeviceExt;

pub struct Image {
    width: usize,
    height: usize,
    texture: wgpu::Texture,
}

impl Image {
    pub fn from_png(png_bytes: &[u8], graphics_device: &mut GraphicsDevice) -> Self {
        let (header, image_data) = png_decoder::decode(png_bytes).expect("Invalid PNG bytes");
        let width = header.width;
        let height = header.height;

        let glyph_texture_extent = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

        let queue = &graphics_device.queue;
        let device = graphics_device.device();

        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some("Image::from_png"),
            size: glyph_texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        };

        let texture = device.create_texture_with_data(queue, &texture_descriptor, &image_data);

        Self { width: header.width as usize, height: header.height as usize, texture }
    }
}
