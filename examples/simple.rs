use glam::vec2;
use simple_game::{
    glam::vec3,
    graphics::{
        text::{AxisAlign, StyledText, TextAlignment, TextSystem},
        DebugDrawer, FrameEncoder, FullscreenQuad, GraphicsDevice, Image, ImageDrawer,
    },
    util::FPSCounter,
    winit::window::Window,
    GameApp,
};

struct SimpleGame {
    fullscreen_quad: FullscreenQuad,
    text_system: TextSystem,
    fps_counter: FPSCounter,
    debug_drawer: DebugDrawer,
    image_drawer: ImageDrawer,
    test_image: Image,
}

impl GameApp for SimpleGame {
    fn init(graphics_device: &mut GraphicsDevice) -> Self {
        Self {
            fullscreen_quad: FullscreenQuad::new(graphics_device),
            text_system: TextSystem::new(graphics_device),
            fps_counter: FPSCounter::new(),
            debug_drawer: DebugDrawer::new(graphics_device),
            image_drawer: ImageDrawer::new(graphics_device),
            test_image: Image::from_png(include_bytes!("resources/grass.png"), graphics_device),
        }
    }

    fn tick(&mut self, _dt: f32) {}

    fn render(&mut self, frame_encoder: &mut FrameEncoder, _window: &Window) {
        self.fullscreen_quad.render(frame_encoder);
        self.text_system.render_horizontal(
            TextAlignment {
                x: AxisAlign::Start(10),
                y: AxisAlign::Start(10),
                max_width: None,
                max_height: None,
            },
            &[StyledText::default_styling(&format!("FPS: {}", self.fps_counter.fps()))],
            frame_encoder,
        );

        let mut shape_recorder = self.debug_drawer.begin();
        shape_recorder.draw_line(vec3(0.0, 0.0, 0.0), vec3(5.0, 5.0, 0.0));
        shape_recorder.draw_circle(vec3(0.0, 0.0, 0.0), 2.0, 0.0);
        shape_recorder.end(frame_encoder);

        let mut image_recorder = self.image_drawer.begin();
        image_recorder.draw_image(&self.test_image, vec2(0.0, 0.0));
        image_recorder.end(frame_encoder);

        self.fps_counter.tick();
    }
}

fn main() {
    simple_game::run_game_app::<SimpleGame>();
}
