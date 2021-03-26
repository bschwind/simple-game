use simple_game::{graphics::FrameEncoder, GameApp};

struct SimpleGame {}

impl GameApp for SimpleGame {
    fn init(&mut self) {
        println!("Init!");
    }

    fn tick(&mut self, _dt: f32) {}

    fn render(&mut self, _frame_encoder: &mut FrameEncoder) {}
}

fn main() {
    let game_app = SimpleGame {};

    simple_game::run_game_app(game_app);
}
