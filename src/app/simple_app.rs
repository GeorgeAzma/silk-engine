use super::renderer;

pub struct SimpleApp;

impl SimpleApp {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self) {}

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {
        gfx.color = [1.0, 0.0, 1.0, 0.5];
        gfx.round_rect(0.0, 0.0, 0.5, 0.5, 1.0);
    }
}
