use crate::bevy::{
    App, BevyGame, Changed, Commands, Component, CoreSchedule, FixedTime, Query, Res, ResMut,
    SimpleGamePlugin, With,
};
use simple_game::{bevy, bevy::IntoSystemAppConfig, graphics::GraphicsDevice};

struct Game {}

impl BevyGame for Game {
    fn init_systems() -> App {
        let mut ecs_world_builder = App::new();

        ecs_world_builder
            .add_plugin(SimpleGamePlugin)
            .insert_resource(FixedTime::new_from_secs(1.0 / Self::desired_fps() as f32))
            .add_startup_system(init_system)
            .add_systems((
                update_game_system.in_schedule(CoreSchedule::FixedUpdate),
                greet,
                render,
                with_change_detection,
            ));

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

fn update_game_system(fixed_time: Res<FixedTime>) {
    println!(
        "Update! Period: {:?}, accumulator: {}",
        fixed_time.period,
        fixed_time.accumulated().as_secs_f32()
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
