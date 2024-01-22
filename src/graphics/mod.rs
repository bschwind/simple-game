use glam::Mat4;
use wgpu::{
    Adapter, Backends, CommandEncoder, CompositeAlphaMode, Device, Instance, InstanceDescriptor,
    Queue, ShaderModuleDescriptor, Surface, SurfaceConfiguration, SurfaceTexture, TextureFormat,
    TextureView,
};
use winit::{dpi::PhysicalSize, window::Window};

mod debug_drawer;
mod fullscreen_quad;
mod image;
mod lines;
mod lines2d;
pub mod text;
mod textured_quad;

pub use debug_drawer::*;
pub use fullscreen_quad::*;
pub use image::*;
pub use lines::*;
pub use lines2d::*;

#[cfg_attr(feature = "bevy", derive(crate::bevy::Resource))]
pub struct GraphicsDevice {
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface,
    surface_config: SurfaceConfiguration,
}

impl GraphicsDevice {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // PRIMARY: All the apis that wgpu offers first tier of support for (Vulkan + Metal + DX12 + Browser WebGPU).
        let instance =
            Instance::new(InstanceDescriptor { backends: Backends::PRIMARY, ..Default::default() });
        let surface =
            unsafe { instance.create_surface(window) }.expect("Failed to create a surface");
        let swapchain_format = wgpu::TextureFormat::Bgra8Unorm;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // Prefer low power when on battery, high performance when on mains.
                power_preference: wgpu::PowerPreference::default(),
                // Indicates that only a fallback adapter can be returned.
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropiate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        Self { adapter, device, queue, surface, surface_config }
    }

    pub fn load_wgsl_shader(device: &Device, shader_src: &str) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_src)),
        })
    }

    pub fn load_spirv_shader(&self, shader_module: ShaderModuleDescriptor) -> wgpu::ShaderModule {
        self.device.create_shader_module(shader_module)
    }

    pub fn begin_frame(&mut self) -> FrameEncoder {
        let frame =
            self.surface.get_current_texture().expect("Failed to acquire next swap chain texture");

        let backbuffer_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let surface_dimensions = self.surface_dimensions();

        FrameEncoder { frame, backbuffer_view, encoder, surface_dimensions }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn surface_dimensions(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn surface_config(&self) -> &SurfaceConfiguration {
        &self.surface_config
    }

    pub fn surface_texture_format(&self) -> TextureFormat {
        self.surface_config.format
    }
}

pub struct FrameEncoder {
    // The `backbuffer_view` field must be listed before the `frame` field.
    // https://github.com/gfx-rs/wgpu/issues/1797
    pub backbuffer_view: TextureView,
    pub frame: SurfaceTexture,
    pub encoder: CommandEncoder,
    surface_dimensions: (u32, u32),
}

impl FrameEncoder {
    pub fn surface_dimensions(&self) -> (u32, u32) {
        self.surface_dimensions
    }
}

pub struct DepthTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl DepthTexture {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self::new_with_format(device, width, height, wgpu::TextureFormat::Depth32Float)
    }

    pub fn new_with_format(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

        let descriptor = wgpu::TextureDescriptor {
            label: Some("Default depth texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&descriptor);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0, // TODO (bschwind) - Needed?
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self { texture, view, sampler }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture.format()
    }

    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.width()
    }
}

// Creates a matrix that projects screen coordinates defined by width and
// height orthographically onto the OpenGL vertex coordinates.
pub fn screen_projection_matrix(width: u32, height: u32) -> Mat4 {
    Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0)
}
