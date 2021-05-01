use simple_game::{
    graphics::{
        text::{AxisAlign, StyledText, TextAlignment, TextSystem},
        FrameEncoder, GraphicsDevice,
    },
    GameApp,
};
use winit::window::Window;

struct SimpleGame {
    text_system: Option<TextSystem>,
}

impl GameApp for SimpleGame {
    fn init(&mut self, graphics_device: &mut GraphicsDevice) {
        self.text_system = Some(TextSystem::new(&graphics_device));
        println!("Init!");
    }

    fn tick(&mut self, _dt: f32) {}

    fn render(&mut self, frame_encoder: &mut FrameEncoder, window: &Window) {
        if let Some(text_system) = &mut self.text_system {
            text_system.render_horizontal(
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
        }
    }
}

fn main() {
    let game_app = SimpleGame { text_system: None };

    simple_game::run_game_app(game_app);
}
