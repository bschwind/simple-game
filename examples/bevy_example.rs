use crate::bevy::{
    App, BevyGame, Changed, Commands, Component, FixedTimestep, FixedTimesteps, Query, Res, ResMut,
    SimpleGamePlugin, SystemSet, With,
};
use simple_game::{bevy, graphics::GraphicsDevice};

const TIMESTEP_LABEL: &str = "game_timestep";

struct Game {}

impl BevyGame for Game {
    fn init_systems() -> App {
        let mut ecs_world_builder = App::new();

        ecs_world_builder
            .add_plugin(SimpleGamePlugin)
            .add_startup_system(init_system)
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        FixedTimestep::step(1.0 / Self::desired_fps() as f64)
                            .with_label(TIMESTEP_LABEL),
                    )
                    .with_system(update_game_system),
            )
            .add_system(greet)
            .add_system(render)
            .add_system(with_change_detection);

        ecs_world_builder
    }
}

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Metallic;

fn greet(query: Query<&Name, With<Metallic>>) {
    for name in query.iter() {
        println!("This is metallic: {}", name.0);
    }
}

fn with_change_detection(query: Query<&Name, Changed<Name>>) {
    // Only get `data` if it changed.
    for data in query.iter() {
        println!("Changed: {}", data.0);
    }
}

fn update_game_system(fixed_timesteps: Res<FixedTimesteps>) {
    let fixed = fixed_timesteps.get(TIMESTEP_LABEL).unwrap();
    println!(
        "Update! Step: {} Step per second: {}, accumulator: {}",
        fixed.step(),
        fixed.steps_per_second(),
        fixed.accumulator()
    );
}

fn render(mut graphics_device: ResMut<GraphicsDevice>) {
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

    frame_encoder.finish();
}

fn init_system(mut commands: Commands) {
    commands.spawn((Name("Car".to_string()), Metallic));
    commands.spawn(Name("Tree".to_string()));
    commands.spawn((Name("Anvil".to_string()), Metallic));
}

fn main() {
    simple_game::bevy::run_bevy_game::<Game>();
}
