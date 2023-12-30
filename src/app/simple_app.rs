use super::renderer;
use rand::prelude::*;
use std::time::{Duration, Instant};

struct InstanceData {
    color: [u8; 4],
    position: [f32; 2],
}

pub struct SimpleApp {
    start_time: Instant,
    elapsed_time: Duration,
    delta_time: Duration,
    instances: Vec<InstanceData>,
}

const INSTANCES: usize = 100;

impl SimpleApp {
    pub fn new() -> Self {
        let mut instances = Vec::with_capacity(INSTANCES);
        let mut rng: StdRng = StdRng::seed_from_u64(1u64);
        for _ in 0..INSTANCES {
            instances.push(InstanceData {
                color: [
                    rng.gen_range(0..255),
                    rng.gen_range(0..255),
                    rng.gen_range(0..255),
                    32,
                ],
                position: [rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)],
            });
        }
        Self {
            start_time: Instant::now(),
            elapsed_time: Duration::from_secs(0),
            delta_time: Duration::from_secs(0),
            instances,
        }
    }

    pub fn update(&mut self) {
        self.delta_time = Instant::now().duration_since(self.start_time) - self.elapsed_time;
        self.elapsed_time = Instant::now().duration_since(self.start_time);
        println!("Dt: {:.1}", self.delta_time.as_secs_f32() * 1000.0);
    }

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {
        for i in 0..self.instances.len() {
            gfx.color = self.instances[i].color;
            gfx.tri(
                self.instances[i].position[0],
                self.instances[i].position[1],
                0.1,
                0.1,
            );
        }
    }
}
