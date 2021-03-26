use wgpu::{
    Adapter, BackendBit, CommandEncoder, Device, Instance, Queue, Surface, SwapChain,
    SwapChainDescriptor, SwapChainTexture,
};
use winit::{dpi::PhysicalSize, window::Window};

mod debug_drawer;
pub use debug_drawer::*;

pub struct GraphicsDevice {
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface,
    swap_chain_descriptor: SwapChainDescriptor,
    swap_chain: SwapChain,
}

impl GraphicsDevice {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // PRIMARY: All the apis that wgpu offers first tier of support for (Vulkan + Metal + DX12 + Browser WebGPU).
        let instance = Instance::new(BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let swapchain_format = wgpu::TextureFormat::Bgra8Unorm;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // Prefer low power when on battery, high performance when on mains.
                power_preference: wgpu::PowerPreference::default(),
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

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

        Self { adapter, device, queue, surface, swap_chain_descriptor, swap_chain }
    }

    pub fn load_shader(&self, shader_src: &'static str) -> wgpu::ShaderModule {
        let mut flags = wgpu::ShaderFlags::VALIDATION;
        match self.adapter().get_info().backend {
            wgpu::Backend::Vulkan | wgpu::Backend::Metal => {
                flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION;
            },
            _ => {},
        }

        self.device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_src)),
            flags,
        })
    }

    pub fn begin_frame(&mut self) -> FrameEncoder {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Failed to acquire next swap chain texture")
            .output;

        let encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        FrameEncoder { queue: &mut self.queue, frame, encoder }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.swap_chain_descriptor.width = new_size.width;
        self.swap_chain_descriptor.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    }

    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn swap_chain_descriptor(&self) -> &SwapChainDescriptor {
        &self.swap_chain_descriptor
    }
}

pub struct FrameEncoder<'a> {
    queue: &'a mut Queue,
    pub frame: SwapChainTexture,
    pub encoder: CommandEncoder,
}

impl<'a> FrameEncoder<'a> {
    pub fn queue(&mut self) -> &mut Queue {
        &mut self.queue
    }

    // TODO(bschwind) - Maybe do this in a Drop impl
    pub fn finish(self) {
        self.queue.submit(Some(self.encoder.finish()));
    }
}
