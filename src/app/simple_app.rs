use super::renderer;
use rand::prelude::*;

pub struct SimpleApp;

impl SimpleApp {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {
        let mut rng: StdRng = StdRng::seed_from_u64(1u64);
        for _ in 0..100 {
            gfx.color = [
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                1.0,
            ];
            gfx.round_rect(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                0.04,
                0.04,
                1.0,
            );
        }
    }
}
