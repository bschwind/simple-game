use crate::bevy::{
    App, BevyGame, Changed, Commands, Component, Fixed, FixedUpdate, Query, Res, ResMut,
    SimpleGamePlugin, Startup, Time, Update, With,
};
use simple_game::{bevy, graphics::GraphicsDevice};

struct Game {}

impl BevyGame for Game {
    fn init_systems() -> App {
        let mut ecs_world_builder = App::new();

        ecs_world_builder
            .add_plugins(SimpleGamePlugin)
            .insert_resource(Time::<Fixed>::from_hz(Self::desired_fps() as f64))
            .add_systems(Startup, init_system)
            .add_systems(FixedUpdate, update_game_system)
            .add_systems(Update, (print_metallic_things, render, with_change_detection));

        ecs_world_builder
    }
}

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Metallic;

fn print_metallic_things(query: Query<&Name, With<Metallic>>) {
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

fn update_game_system(fixed_time: Res<Time<Fixed>>) {
    println!(
        "Update! Period: {:?}, accumulator: {}",
        fixed_time.delta(),
        fixed_time.overstep().as_secs_f32()
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
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    graphics_device.queue().submit(Some(frame_encoder.encoder.finish()));
    frame_encoder.frame.present();
}

fn init_system(mut commands: Commands) {
    commands.spawn((Name("Car".to_string()), Metallic));
    commands.spawn(Name("Tree".to_string()));
    commands.spawn((Name("Anvil".to_string()), Metallic));
}

fn main() {
    simple_game::bevy::run_bevy_game::<Game>();
}
