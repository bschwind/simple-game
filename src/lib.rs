use crate::graphics::GraphicsDevice;
use std::time::{Duration, Instant};
use thiserror::Error;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::{Fullscreen, Window, WindowBuilder},
};

pub mod graphics;
pub mod util;

#[cfg(feature = "bevy")]
pub mod bevy;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Event loop error: {0}")]
    EventLoopError(#[from] winit::error::EventLoopError),

    #[error("Window building error: {0}")]
    WindowBuilderError(#[from] winit::error::OsError),
}

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

    // TODO(bschwind) - Separate tick rate from render rate.
    fn desired_fps() -> RefreshRate {
        RefreshRate::Monitor
    }

    fn handle_window_event(&mut self, event: &WindowEvent, event_loop: &EventLoopWindowTarget<()>) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
        }
    }

    fn init(graphics_device: &mut GraphicsDevice) -> Self;

    fn resize(&mut self, _graphics_device: &mut GraphicsDevice, _width: u32, _height: u32) {}
    fn tick(&mut self, dt: f32);
    fn render(&mut self, graphics_device: &mut GraphicsDevice, window: &Window);
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RefreshRate {
    Monitor,
    Fps(usize),
}

async fn run<G: 'static + GameApp>() -> Result<(), Error> {
    let event_loop = EventLoop::new()?;

    let window =
        {
            let window_builder = WindowBuilder::new().with_title(G::window_title());

            let window_builder =
                match G::window_dimensions() {
                    WindowDimensions::Windowed(width, height) => {
                        window_builder.with_inner_size(PhysicalSize::new(width, height))
                    },
                    WindowDimensions::FullScreen => {
                        window_builder.with_fullscreen(Some(Fullscreen::Borderless(None)))
                    },
                };

            window_builder.build(&event_loop)?
        };

    let frame_dt = match G::desired_fps() {
        RefreshRate::Monitor => {
            let monitor = window
                .current_monitor()
                .expect("Requested monitor refresh rate, but can't fetch window.current_monitor()");
            let refresh_rate_millihertz = monitor.refresh_rate_millihertz().unwrap_or(60_000);

            Duration::from_micros((1000000000.0 / refresh_rate_millihertz as f64) as u64)
        },
        RefreshRate::Fps(fps) => Duration::from_micros((1000000.0 / fps as f64) as u64),
    };

    let mut graphics_device = GraphicsDevice::new(&window).await;

    let mut game_app = G::init(&mut graphics_device);

    let mut last_frame_time = Instant::now();

    event_loop.run(move |event, window_target| {
        match event {
            Event::AboutToWait => {
                window.request_redraw();
            },
            Event::WindowEvent { event: WindowEvent::Resized(new_size), .. } => {
                graphics_device.resize(new_size);
                game_app.resize(&mut graphics_device, new_size.width, new_size.height);
            },
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                if last_frame_time.elapsed() >= frame_dt {
                    let now = Instant::now();
                    last_frame_time = now;

                    // TODO(bschwind) - Decouple game update ticks and rendering ticks.
                    game_app.tick(frame_dt.as_secs_f32());
                    game_app.render(&mut graphics_device, &window);
                }

                window.request_redraw();
            },
            Event::WindowEvent { event, .. } => {
                if let WindowEvent::CloseRequested = event {
                    window_target.exit();
                }

                game_app.handle_window_event(&event, window_target);
            },
            _ => (),
        }
    })?;

    Ok(())
}

pub fn run_game_app<G: 'static + GameApp>() -> Result<(), Error> {
    pollster::block_on(run::<G>())?;

    Ok(())
}
