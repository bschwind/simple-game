use crate::{graphics::GraphicsDevice, WindowDimensions};
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, WindowBuilder},
};

pub use bevy_app::*;
pub use bevy_core::*;
pub use bevy_ecs::{self, prelude::*, *};

pub trait BevyGame {
    fn window_title() -> &'static str {
        "Simple Game"
    }

    fn window_dimensions() -> WindowDimensions {
        WindowDimensions::Windowed(1280, 720)
    }

    fn desired_fps() -> usize {
        60
    }

    fn init_systems() -> AppBuilder;
}

pub trait HeadlessBevyGame {
    fn desired_fps() -> usize {
        60
    }

    fn init_systems() -> AppBuilder;
}

async fn run<G: 'static + BevyGame>() {
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

    let graphics_device = GraphicsDevice::new(&window).await;
    let mut game_app_builder = G::init_systems();
    game_app_builder.add_event::<KeyboardInput>();
    let mut game_app = std::mem::take(&mut game_app_builder.app);

    game_app.world.insert_resource(graphics_device);

    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            game_app.update();
        },
        Event::WindowEvent { event: WindowEvent::Resized(new_size), .. } => {
            let mut graphics_device = game_app.world.get_resource_mut::<GraphicsDevice>().unwrap();
            graphics_device.resize(new_size);
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
            WindowEvent::KeyboardInput { ref input, .. } => {
                let mut keyboard_input_events =
                    game_app.world.get_resource_mut::<Events<KeyboardInput>>().unwrap();

                keyboard_input_events.send(*input);
            },
            _ => (),
        },
        _ => (),
    });
}

async fn run_headless<G: 'static + HeadlessBevyGame>() {
    let mut game_app_builder = G::init_systems();
    let mut game_app = std::mem::take(&mut game_app_builder.app);

    loop {
        game_app.update();
    }
}

pub fn run_bevy_game<G: 'static + BevyGame>() {
    pollster::block_on(run::<G>());
}

pub fn run_headless_bevy_game<G: 'static + HeadlessBevyGame>() {
    pollster::block_on(run_headless::<G>());
}
