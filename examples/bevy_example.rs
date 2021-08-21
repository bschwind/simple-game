use crate::bevy::{
    App, AppBuilder, BevyGame, Changed, Commands, CorePlugin, FixedTimestep, FixedTimesteps, Query,
    Res, ResMut, SystemSet, With,
};
use simple_game::{bevy, bevy::IntoSystem, graphics::GraphicsDevice};

struct Game {}

impl BevyGame for Game {
    fn init_systems() -> AppBuilder {
        let mut ecs_world_builder = App::build();

        ecs_world_builder
            .add_plugin(CorePlugin)
            .add_startup_system(init_system.system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        FixedTimestep::step(1.0 / Self::desired_fps() as f64)
                            .with_label("game_timestep"),
                    )
                    .with_system(update_game_system.system()),
            )
            .add_system(greet.system())
            .add_system(render.system())
            .add_system(with_change_detection.system());

        ecs_world_builder
    }
}

struct Name(String);
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
    let fixed = fixed_timesteps.get("game_timestep").unwrap();
    println!(
        "Update! Step: {} Step per second: {}, accumulator: {}",
        fixed.step,
        fixed.steps_per_second(),
        fixed.accumulator
    );
}

fn render(mut graphics_device: ResMut<GraphicsDevice>) {
    let mut frame_encoder = graphics_device.begin_frame();

    {
        let encoder = &mut frame_encoder.encoder;

        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Screen Clear"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame_encoder.backbuffer_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
    }

    frame_encoder.finish();
}

fn init_system(mut commands: Commands) {
    commands.spawn().insert(Name("Car".to_string())).insert(Metallic);
    commands.spawn().insert(Name("Tree".to_string()));
    commands.spawn().insert(Name("Anvil".to_string())).insert(Metallic);
}

fn main() {
    simple_game::bevy::run_bevy_game::<Game>();
}
