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
            unsafe { instance.create_surface(window).expect("Failed to create a surface") };
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

        FrameEncoder {
            device: &self.device,
            queue: &self.queue,
            frame,
            backbuffer_view,
            encoder,
            surface_dimensions,
        }
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

pub struct FrameEncoder<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    // The `backbuffer_view` field must be listed before the `frame` field.
    // https://github.com/gfx-rs/wgpu/issues/1797
    pub backbuffer_view: TextureView,
    pub frame: SurfaceTexture,
    pub encoder: CommandEncoder,
    surface_dimensions: (u32, u32),
}

impl<'a> FrameEncoder<'a> {
    pub fn queue(&self) -> &Queue {
        self.queue
    }

    // TODO(bschwind) - Maybe do this in a Drop impl
    pub fn finish(self) {
        self.queue.submit(Some(self.encoder.finish()));
        self.frame.present();
    }

    pub fn surface_dimensions(&self) -> (u32, u32) {
        self.surface_dimensions
    }
}

// Creates a matrix that projects screen coordinates defined by width and
// height orthographically onto the OpenGL vertex coordinates.
pub fn screen_projection_matrix(width: u32, height: u32) -> Mat4 {
    Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0)
}
