use super::renderer;
use rand::prelude::*;
use std::time::{Duration, Instant};

pub struct SimpleApp {
    start_time: Instant,
    elapsed_time: Duration,
    delta_time: Duration,
}

impl SimpleApp {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            elapsed_time: Duration::from_secs(0),
            delta_time: Duration::from_secs(0),
        }
    }

    pub fn update(&mut self) {
        self.delta_time = Instant::now().duration_since(self.start_time) - self.elapsed_time;
        self.elapsed_time = Instant::now().duration_since(self.start_time);
        std::thread::sleep(Duration::from_millis(16));
    }

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {
        let mut rng: StdRng = StdRng::seed_from_u64(1u64);
        for _ in 0..30 {
            gfx.rotation = rng.gen_range(-3.1415..3.1415) + self.elapsed_time.as_secs_f32() * 0.2;
            gfx.color = [
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                1.0,
            ];
            gfx.tri(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 0.1, 0.1);
        }
    }
}
