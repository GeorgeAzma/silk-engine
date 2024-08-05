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

    pub fn render(&mut self, gfx: &mut renderer::Renderer) {
        let app = unsafe { self.app.as_ref().unwrap() };
        // gfx.text(
        //     "`1234567890\n-=qwerty\nuiop[]\\asdfgh\njkl;'zxcvbnm,./\n!@#$%^&*()_\n+QWERTY\nUIOP{}|ASDFGH\nJKL:\"ZXCVBNM<>?",
        //     -0.95,
        //     0.8,
        //     0.1,
        // );
        gfx.rotation = 0.5 + app.time;
        gfx.text("AAA", 0.5, 0.0, 0.25);

        // Green
        gfx.color = [0, 255, 0, 128];
        gfx.rect(0.5, 0.0, 0.5, 0.5);

        // Pink
        gfx.color = [255, 128, 255, 150];
        gfx.circle(0.3, -0.4, 0.2);

        // Blue
        gfx.color = [64, 0, 255, 100];
        gfx.circle(0.3, -0.2, 0.15);
    }
}
