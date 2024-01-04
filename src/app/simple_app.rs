use super::renderer;
use rand::prelude::*;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::event::{MouseButton, WindowEvent};
use winit::keyboard::NamedKey;
use winit::window::Window;

pub struct SimpleApp {
    pub window: Rc<Window>,
    pub time: f32,
    pub dt: f32,
    pub width: u32,
    pub height: u32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_left: bool,
    pub mouse_middle: bool,
    pub mouse_right: bool,
    pub key: [bool; NamedKey::F35 as usize + 1],
}

impl SimpleApp {
    pub fn update(&mut self) {
        // println!("Dt: {:.1}", self.delta_time.as_secs_f32() * 1000.0);
    }

    pub fn event(&mut self, _event: &WindowEvent) {}

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {}
}
