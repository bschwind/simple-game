use crate::{graphics::GraphicsDevice, Error, WindowDimensions};
use bevy_time::TimePlugin;
use winit::{
    dpi::PhysicalSize,
    event::{Event as WinitEvent, KeyEvent as WinitKeyboardInput, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::{Fullscreen, WindowBuilder},
};

pub use bevy_app::{self, prelude::*};
pub use bevy_core::*;
pub use bevy_ecs::{self, prelude::*};
pub use bevy_time::{self, prelude::*};
pub use bevy_transform::{self, prelude::*};

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

    fn init_systems() -> App;
}

pub trait HeadlessBevyGame {
    fn desired_fps() -> usize {
        60
    }

    fn init_systems() -> App;
}

pub struct SimpleGamePlugin;

impl Plugin for SimpleGamePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(TaskPoolPlugin::default());
        app.add_plugins(TypeRegistrationPlugin);
        app.add_plugins(FrameCountPlugin);
        app.add_plugins(TimePlugin);
        // TODO(bschwind) - ScheduleRunnerPlugin might be needed as well.
    }
}

async fn run<G: 'static + BevyGame>() -> Result<(), crate::Error> {
    let event_loop = EventLoop::new()?;

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
    let mut game_app = G::init_systems();
    game_app.add_event::<KeyboardInput>();

    game_app.world.insert_resource(graphics_device);

    event_loop.run(move |event, window_target| match event {
        WinitEvent::AboutToWait => {
            game_app.update();
        },
        WinitEvent::WindowEvent { event: WindowEvent::Resized(new_size), .. } => {
            let mut graphics_device = game_app.world.get_resource_mut::<GraphicsDevice>().unwrap();
            graphics_device.resize(new_size);
        },
        WinitEvent::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => {
                window_target.exit();
            },
            WindowEvent::KeyboardInput {
                event: WinitKeyboardInput { logical_key: Key::Named(NamedKey::Escape), .. },
                ..
            } => {
                window_target.exit();
            },
            WindowEvent::KeyboardInput { ref event, .. } => {
                let mut keyboard_input_events =
                    game_app.world.get_resource_mut::<Events<KeyboardInput>>().unwrap();

                // TODO(bschwind) - Avoid the clone() if possible.
                keyboard_input_events.send(KeyboardInput(event.clone()));
            },
            _ => (),
        },
        _ => (),
    })?;

    Ok(())
}

async fn run_headless<G: 'static + HeadlessBevyGame>() {
    let mut game_app = G::init_systems();
    let runner = std::mem::replace(&mut game_app.runner, Box::new(game_runner));
    (runner)(game_app);
}

fn game_runner(mut app: App) {
    app.update();
}

pub fn run_bevy_game<G: 'static + BevyGame>() -> Result<(), Error> {
    pollster::block_on(run::<G>())?;

    Ok(())
}

pub fn run_headless_bevy_game<G: 'static + HeadlessBevyGame>() {
    pollster::block_on(run_headless::<G>());
}

#[derive(Debug, Event)]
pub struct KeyboardInput(WinitKeyboardInput);

impl AsRef<WinitKeyboardInput> for KeyboardInput {
    fn as_ref(&self) -> &WinitKeyboardInput {
        &self.0
    }
}
