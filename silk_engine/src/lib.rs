pub use std::{
    collections::{HashMap, HashSet},
    process::abort,
    ptr::{self, null, null_mut},
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use lazy_static::lazy_static;
use winit::window::WindowAttributes;
use winit::window::WindowId;
use winit::{event_loop::ActiveEventLoop, window::Window};

mod input;
pub mod print;
use input::*;
pub use input::{Event, Key, Mouse};
pub use print::*;
mod gfx;
pub use gfx::*;
mod util;
mod window;
pub use window::*;

pub trait App: Sized {
    fn new(app: Arc<AppContext<Self>>) -> Self;
    fn update(&mut self);
    fn render(&mut self);
    fn event(&mut self, _e: Event) {}
}

pub struct AppContext<T: App> {
    my_app: Option<T>,
    pub window: Arc<Window>,
    pub width: u32,
    pub height: u32,
    pub start_time: std::time::Instant,
    pub time: f32,
    pub dt: f32,
    pub frame: u32,
    input: Input,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_scroll: f32,
    renderer: Renderer,
}

impl<T: App> AppContext<T> {
    pub fn new() -> *mut Self {
        scope_time!("init");
        let window = WINDOW.read().unwrap().as_ref().unwrap().clone();
        let width = window.inner_size().width;
        let height = window.inner_size().height;

        let app = Arc::new(Self {
            my_app: None,
            window: window.clone(),
            width,
            height,
            start_time: Instant::now(),
            time: 0.0,
            dt: 0.0,
            frame: 0,
            input: Input::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_scroll: 0.0,
            renderer: Renderer::new(),
        });
        let app_mut = ptr::from_ref(app.as_ref()).cast_mut();
        unsafe { app_mut.as_mut() }.unwrap().my_app = Some(T::new(app.clone()));
        app_mut
    }

    fn update(&mut self) {
        scope_time!("update {}", self.frame; self.frame < 4);
        let now = Instant::now().duration_since(self.start_time).as_secs_f32();
        self.dt = now - self.time;
        self.time = now;

        let uniform_data = GlobalUniform {
            resolution: [self.width, self.height],
            mouse_pos: [self.mouse_x, self.mouse_y],
            time: self.time,
            dt: self.dt,
        };
        buffer_alloc().copy(ctx().buffer("global uniform"), &uniform_data);
        self.my_app().update();
    }

    fn render(&mut self) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        scope_time!("render {}", self.frame; self.frame < 4);

        self.renderer.begin_render();

        self.my_app().render();

        self.renderer.end_render();

        self.input.reset();
        self.frame += 1;
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == self.width || height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        if width == 0 || height == 0 {
            return;
        }
        recreate_swapchain();
    }

    fn event(&mut self, event_loop: &ActiveEventLoop, event: Event, window_id: WindowId) {
        if window_id == self.window.id() {
            self.input.event(&event, self.width, self.height);
            self.mouse_x = self.input.mouse_x();
            self.mouse_y = self.input.mouse_y();
            self.mouse_scroll = self.input.mouse_scroll();
            match &event {
                Event::Resized(size) => {
                    self.resize(size.width, size.height);
                }
                Event::RedrawRequested => {
                    self.update();
                    self.render();
                }
                Event::Focused(focused) => {
                    if !*focused {
                        self.input.reset();
                    }
                }
                Event::CloseRequested => event_loop.exit(),
                _ => {}
            }
        }

        self.my_app().event(event);
        self.window.request_redraw();
    }

    fn my_app(&mut self) -> &mut T {
        self.my_app.as_mut().unwrap()
    }

    expose!(input.[mouse_press_x, mouse_press_y, mouse_drag_x, mouse_drag_y](m: Mouse) -> f32);
    expose!(input.[mouse_down, mouse_released, mouse_pressed](m: Mouse) -> bool);
    expose!(input.[key_down, key_released, key_pressed](k: Key) -> bool);
    expose!(input.focused() -> bool);
}

pub struct Engine<T: App> {
    app: Option<*mut AppContext<T>>,
}

impl<T: App> Engine<T> {
    pub fn new() -> Engine<T> {
        let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let mut engine = Self { app: None };
        event_loop.run_app(&mut engine).unwrap();
        Self { app: None }
    }
}

impl<T: App> winit::application::ApplicationHandler for Engine<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        *WINDOW.write().unwrap() = Some(Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        ));
        self.app = Some(AppContext::new());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: Event,
    ) {
        if let Some(app) = unsafe { self.app.unwrap().as_mut() } {
            app.event(event_loop, event, window_id);
        }
    }
}
