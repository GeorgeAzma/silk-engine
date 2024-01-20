#![allow(dead_code, unused_imports, unused_variables)]
use super::renderer;
use super::App;
use std::rc::Rc;
use winit::event::{MouseButton, WindowEvent};
use winit::keyboard::KeyCode;
use winit::window::Window;

pub struct SimpleApp {
    pub app: *const App,
}

impl SimpleApp {
    pub fn new(app: &App) -> Self {
        Self { app }
    }

    pub fn update(&mut self) {
        let app = unsafe { self.app.as_ref().unwrap() };
    }

    pub fn event(&mut self, event: &WindowEvent) {}

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {}
}
