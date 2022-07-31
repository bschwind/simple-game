use crate::{graphics::Image, FrameEncoder};
use glam::Vec2;

pub struct Screen {}

impl Screen {
    pub fn new() -> Self {
        Self {}
    }

    pub fn begin(&mut self) -> DrawRecorder {
        DrawRecorder { screen: self }
    }
}

pub struct DrawRecorder<'a> {
    screen: &'a mut Screen,
}

impl DrawRecorder<'_> {
    pub fn draw_line(&mut self, start: Vec2, end: Vec2) {}

    pub fn draw_circle(&mut self, center: Vec2, radius: f32, rotation: f32) {}

    pub fn draw_text(&mut self, text: &str) {}

    pub fn draw_image(&mut self, image: &Image) {}

    pub fn end(self, frame_encoder: &mut FrameEncoder) {}
}
