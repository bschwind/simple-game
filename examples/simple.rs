use glam::vec3;
use simple_game::{
    graphics::{
        text::{AxisAlign, StyledText, TextAlignment, TextSystem},
        DebugDrawer, FrameEncoder, GraphicsDevice,
    },
    GameApp,
};
use winit::window::Window;

struct SimpleGame {
    text_system: TextSystem,
    debug_drawer: DebugDrawer,
}

impl GameApp for SimpleGame {
    fn init(graphics_device: &mut GraphicsDevice) -> Self {
        println!("Init!");

        Self {
            text_system: TextSystem::new(&graphics_device),
            debug_drawer: DebugDrawer::new(&graphics_device),
        }
    }

    fn tick(&mut self, _dt: f32) {}

    fn render(&mut self, frame_encoder: &mut FrameEncoder, window: &Window) {
        self.text_system.render_horizontal(
            TextAlignment {
                x: AxisAlign::Start(10),
                y: AxisAlign::WindowCenter,
                max_width: None,
                max_height: None,
            },
            &[StyledText::default_styling("This is a test.")],
            frame_encoder,
            window.inner_size(),
        );

        let mut shape_recorder = self.debug_drawer.begin();

        shape_recorder.draw_line(vec3(0.0, 0.0, 0.0), vec3(5.0, 5.0, 0.0));
        shape_recorder.end(frame_encoder);
    }
}

fn main() {
    simple_game::run_game_app::<SimpleGame>();
}
