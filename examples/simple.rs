use glam::{vec2, vec3};
use simple_game::{
    graphics::{
        text::{AxisAlign, StyledText, TextAlignment, TextSystem},
        DebugDrawer, FrameEncoder, FullscreenQuad, GraphicsDevice, Image, ImageDrawer,
        LineDrawer2d, LineVertex,
    },
    util::FPSCounter,
    GameApp,
};
use winit::window::Window;

struct SimpleGame {
    fullscreen_quad: FullscreenQuad,
    text_system: TextSystem,
    fps_counter: FPSCounter,
    debug_drawer: DebugDrawer,
    image_drawer: ImageDrawer,
    line_drawer: LineDrawer2d,
    test_image: Image,
    circles: Vec<LineVertex>,
}

impl GameApp for SimpleGame {
    fn init(graphics_device: &mut GraphicsDevice) -> Self {
        const CIRCLE_SEGMENTS: usize = 100;
        let radius = 200.0;
        let line_width = 40.0;

        let mut circles = vec![];
        for i in 0..CIRCLE_SEGMENTS {
            let frac_1 = (i as f32 / CIRCLE_SEGMENTS as f32) * 2.0 * std::f32::consts::PI;
            let frac_2 = ((i + 1) as f32 / CIRCLE_SEGMENTS as f32) * 2.0 * std::f32::consts::PI;

            circles.push(LineVertex::new(
                radius * vec2(frac_1.cos(), frac_1.sin()) + vec2(300.0, 300.0),
                line_width,
            ));
            circles.push(LineVertex::new(
                radius * vec2(frac_2.cos(), frac_2.sin()) + vec2(300.0, 300.0),
                line_width,
            ));
        }

        circles.push(LineVertex::new(vec2(500.0, 300.0), 5.0));

        for i in 0..500 {
            let thickness = 5.0 + (i as f32 * 0.3);
            circles.push(LineVertex::new(
                vec2(700.0, 500.0) + vec2(i as f32 * 3.0, ((i as f32) * 0.06).sin() * 100.0),
                thickness,
            ));
        }

        let (screen_width, screen_height) = graphics_device.surface_dimensions();
        let surface_texture_format = graphics_device.surface_texture_format();

        Self {
            fullscreen_quad: FullscreenQuad::new(graphics_device.device(), surface_texture_format),
            text_system: TextSystem::new(
                graphics_device.device(),
                surface_texture_format,
                screen_width,
                screen_height,
            ),
            fps_counter: FPSCounter::new(),
            debug_drawer: DebugDrawer::new(
                graphics_device.device(),
                surface_texture_format,
                screen_width,
                screen_height,
            ),
            image_drawer: ImageDrawer::new(
                graphics_device.device(),
                surface_texture_format,
                screen_width,
                screen_height,
            ),
            line_drawer: LineDrawer2d::new(
                graphics_device.device(),
                surface_texture_format,
                screen_width,
                screen_height,
            ),
            test_image: Image::from_png(
                include_bytes!("resources/grass.png"),
                graphics_device.device(),
                graphics_device.queue(),
            ),
            circles,
        }
    }

    fn resize(&mut self, _graphics_device: &mut GraphicsDevice, width: u32, height: u32) {
        self.debug_drawer.resize(width, height);
        self.image_drawer.resize(width, height);
        self.line_drawer.resize(width, height);
        self.text_system.resize(width, height);
    }

    fn tick(&mut self, _dt: f32) {}

    fn render(&mut self, frame_encoder: &mut FrameEncoder, _window: &Window) {
        self.fullscreen_quad.render(&mut frame_encoder.encoder, &frame_encoder.backbuffer_view);
        self.text_system.render_horizontal(
            TextAlignment {
                x: AxisAlign::Start(10),
                y: AxisAlign::Start(10),
                max_width: None,
                max_height: None,
            },
            &[StyledText::default_styling(&format!("FPS: {}", self.fps_counter.fps()))],
            &mut frame_encoder.encoder,
            &frame_encoder.backbuffer_view,
            frame_encoder.queue,
        );

        let mut shape_recorder = self.debug_drawer.begin();
        shape_recorder.draw_line(vec3(0.0, 0.0, 0.0), vec3(5.0, 5.0, 0.0));
        shape_recorder.draw_circle(vec3(0.0, 0.0, 0.0), 2.0, 0.0);
        shape_recorder.end(
            &mut frame_encoder.encoder,
            &frame_encoder.backbuffer_view,
            frame_encoder.queue,
        );

        let mut image_recorder = self.image_drawer.begin();
        image_recorder.draw_image(&self.test_image, vec2(0.0, 0.0));
        image_recorder.end(
            &mut frame_encoder.encoder,
            &frame_encoder.backbuffer_view,
            frame_encoder.queue,
        );

        let mut line_recorder = self.line_drawer.begin();
        line_recorder.draw_round_line_strip(&self.circles);
        line_recorder.end(
            &mut frame_encoder.encoder,
            &frame_encoder.backbuffer_view,
            frame_encoder.queue,
        );

        self.fps_counter.tick();
    }
}

fn main() {
    simple_game::run_game_app::<SimpleGame>();
}
