use super::renderer;
use super::App;
use std::rc::Rc;
use winit::event::{MouseButton, WindowEvent};
use winit::keyboard::NamedKey;
use winit::window::Window;

pub struct SimpleApp {
    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
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
    images: Vec<Rc<renderer::image::Image>>,
}

impl SimpleApp {
    pub fn new(app: &App) -> Self {
        Self {
            device: app.device.clone(),
            queue: app.queue.clone(),
            window: app.window.clone(),
            time: 0.0,
            dt: 0.0,
            width: app.size.width,
            height: app.size.height,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left: false,
            mouse_right: false,
            mouse_middle: false,
            key: [false; NamedKey::F35 as usize + 1],
            images: vec![],
        }
    }

    pub fn update(&mut self) {}

    pub fn event(&mut self, _event: &WindowEvent) {}

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {}
}
