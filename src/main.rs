pub use std::{
    collections::{HashMap, HashSet},
    ptr,
    ptr::{null, null_mut},
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use winit::error::EventLoopError;
use winit::window::WindowId;
use winit::{event_loop::ActiveEventLoop, window::Window};

mod input;
pub mod print;
use input::*;
pub use input::{Event, Key, Mouse};
pub use print::*;
mod pipeline;
mod shader;
mod util;
pub use util::*;
mod vulkan;
pub use vulkan::*;
mod window;
use window::*;
mod app;
use app::MyApp;
mod renderer;
use renderer::*;
mod buffer_alloc;
mod cmd_alloc;
mod desc_alloc;
mod dsl_manager;
mod pipeline_layout_manager;
mod render_context;

pub struct App {
    my_app: Option<MyApp>,
    window: Arc<Window>,
    windows: Vec<WindowData>, // windows[0] = window (main)
    width: u32,               // main window width
    height: u32,              // main window height
    start_time: std::time::Instant,
    time: f32,
    dt: f32,
    frame: u32,
    input: Input,
    mouse_x: f32,
    mouse_y: f32,
    mouse_scroll: f32,
    renderer: Renderer,
}

impl App {
    pub fn new(event_loop: &ActiveEventLoop) -> *mut Self {
        scope_time!("init");
        let window_data = WindowData::new(event_loop);
        let width = window_data.window.inner_size().width;
        let height = window_data.window.inner_size().height;

        let app = Arc::new(Self {
            my_app: None,
            window: window_data.window.clone(),
            windows: vec![window_data],
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
        unsafe { app_mut.as_mut() }.unwrap().my_app = Some(MyApp::new(app.clone()));
        app_mut
    }

    fn update(&mut self) {
        scope_time!("update {}", self.frame; self.frame < 4);
        let now = Instant::now().duration_since(self.start_time).as_secs_f32();
        self.dt = now - self.time;
        self.time = now;

        let ubo_data = Uniform {
            resolution: [self.width, self.height],
            mouse_pos: [self.mouse_x, self.mouse_y],
            time: self.time,
            dt: self.dt,
        };
        BUFFER_ALLOC
            .lock()
            .unwrap()
            .copy(*UNIFORM_BUFFER, &ubo_data);
        self.my_app().update();
    }

    fn render(&mut self) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        scope_time!("render {}", self.frame; self.frame < 4);

        self.renderer.begin_render(&self.windows[0]);

        self.my_app().render();

        self.renderer.end_render(&self.windows[0]);

        self.input.reset();
        self.frame += 1;
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || width == self.width || height == self.height {
            return;
        }
        scope_time!("resize {width}x{height}");
        self.width = width;
        self.height = height;
        self.windows[0].recreate_swapchain();
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

    fn my_app(&mut self) -> &mut MyApp {
        self.my_app.as_mut().unwrap()
    }

    expose!(input.[mouse_press_x, mouse_press_y, mouse_drag_x, mouse_drag_y](m: Mouse) -> f32);
    expose!(input.[mouse_down, mouse_released, mouse_pressed](m: Mouse) -> bool);
    expose!(input.[key_down, key_released, key_pressed](k: Key) -> bool);
    expose!(input.focused() -> bool);
}

#[derive(Default)]
struct AppBuilder {
    app: Option<*mut App>,
}

impl winit::application::ApplicationHandler for AppBuilder {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.app = Some(App::new(event_loop));
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

fn main() -> Result<(), EventLoopError> {
    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = AppBuilder::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
