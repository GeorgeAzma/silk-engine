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
        // println!("Dt: {:.1}", self.delta_time.as_secs_f32() * 1000.0);
    }

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {}
}
