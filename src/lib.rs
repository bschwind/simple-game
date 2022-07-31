use crate::graphics::{FrameEncoder, GraphicsDevice};
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowBuilder},
};

pub mod graphics;
pub mod util;

pub use glam;
pub use wgpu;
pub use winit;

#[cfg(feature = "bevy")]
pub mod bevy;

pub enum WindowDimensions {
    Windowed(u32, u32),
    FullScreen,
}

pub trait GameApp {
    fn window_title() -> &'static str {
        "Simple Game"
    }

    fn window_dimensions() -> WindowDimensions {
        WindowDimensions::Windowed(1280, 720)
    }

    fn desired_fps() -> usize {
        60
    }

    fn init(graphics_device: &mut GraphicsDevice) -> Self;

    fn resize(&mut self, _width: u32, _height: u32) {}
    fn tick(&mut self, dt: f32);
    fn render(&mut self, frame_encoder: &mut FrameEncoder, window: &Window);
}

async fn run<G: 'static + GameApp>() {
    let event_loop = EventLoop::new();

    let window = {
        let window_builder = WindowBuilder::new().with_title(G::window_title());

        let window_builder = match G::window_dimensions() {
            WindowDimensions::Windowed(width, height) => {
                window_builder.with_inner_size(PhysicalSize::new(width, height))
            },
            WindowDimensions::FullScreen => {
                window_builder.with_fullscreen(Some(Fullscreen::Borderless(None)))
            },
        };

        window_builder.build(&event_loop).unwrap()
    };

    let frame_dt = Duration::from_micros((1000000.0 / G::desired_fps() as f64) as u64);

    let mut graphics_device = GraphicsDevice::new(&window).await;

    let mut game_app = G::init(&mut graphics_device);

    let mut last_frame_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                if last_frame_time.elapsed() >= frame_dt {
                    let now = Instant::now();
                    last_frame_time = now;

                    game_app.tick(frame_dt.as_secs_f32());
                    window.request_redraw();
                }
            },
            Event::WindowEvent { event: WindowEvent::Resized(new_size), .. } => {
                graphics_device.resize(new_size);
                game_app.resize(new_size.width, new_size.height);
            },
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                },
                WindowEvent::KeyboardInput {
                    input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. },
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                },
                _ => (),
            },
            Event::RedrawRequested(_window_id) => {
                // Draw the scene
                let mut frame_encoder = graphics_device.begin_frame();

                {
                    let encoder = &mut frame_encoder.encoder;

                    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Screen Clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &frame_encoder.backbuffer_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });
                }

                game_app.render(&mut frame_encoder, &window);
                frame_encoder.finish();
            },
            _ => (),
        }
    });
}

pub fn run_game_app<G: 'static + GameApp>() {
    pollster::block_on(run::<G>());
}
