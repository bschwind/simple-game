use crate::graphics::{FrameEncoder, GraphicsDevice};
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, WindowBuilder},
};

pub mod graphics;

pub use wgpu;
pub use winit;

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

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn init(&mut self);
    fn tick(&mut self, dt: f32);
    fn render(&mut self, frame_encoder: &mut FrameEncoder);
}

async fn run<G: 'static + GameApp>(mut game_app: G) {
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

    game_app.init();

    let frame_dt = Duration::from_micros((1000000.0 / G::desired_fps() as f64) as u64);

    let mut graphics_device = GraphicsDevice::new(&window).await;
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

                window.request_redraw();
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
                game_app.render(&mut frame_encoder);
                frame_encoder.finish();
            },
            _ => (),
        }
    });
}

pub fn run_game_app<G: 'static + GameApp>(game_app: G) {
    pollster::block_on(run(game_app));
}
