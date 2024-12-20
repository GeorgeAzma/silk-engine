use std::sync::LazyLock;
pub use std::{
    collections::{HashMap, HashSet},
    process::abort,
    ptr::{self, null, null_mut},
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use window::WindowContext;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event_loop::ActiveEventLoop,
    monitor::MonitorHandle,
    window::Window,
};
use winit::{event_loop::ControlFlow, window::WindowId};
use winit::{platform::run_on_demand::EventLoopExtRunOnDemand, window::WindowAttributes};

mod input;
pub mod print;
use input::*;
pub use input::{Event, Key, Mouse};
pub use print::*;
mod gfx;
pub use gfx::*;
mod util;
mod window;

pub trait App: Sized {
    fn new(app: *mut AppContext<Self>) -> Self;
    fn update(&mut self);
    fn render(&mut self);
    fn event(&mut self, _e: Event) {}
}

pub struct AppContext<A: App> {
    my_app: Option<A>,
    pub window: Window,
    pub width: u32,
    pub height: u32,
    pub monitor: MonitorHandle,
    pub monitor_width: u32,
    pub monitor_height: u32,
    pub refresh_rate: u32,
    pub start_time: std::time::Instant,
    pub time: f32,
    pub dt: f32,
    pub frame: u32,
    input: Input,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_scroll: f32,
    window_ctx: Arc<Mutex<WindowContext>>,
    render_ctx: Arc<Mutex<RenderContext>>,
    renderer: Renderer,
}

impl<A: App> AppContext<A> {
    pub fn new(window: Window, monitor: MonitorHandle) -> Arc<Mutex<Self>> {
        scope_time!("init");
        let PhysicalSize { width, height } = window.inner_size();
        let PhysicalSize {
            width: monitor_width,
            height: monitor_height,
        } = monitor.size();
        let refresh_rate =
            (monitor.refresh_rate_millihertz().unwrap_or(60) as f32 / 1000.0).round() as u32;
        log!("Monitor: {}", monitor.name().unwrap_or_default());

        let render_ctx = Arc::new(Mutex::new(RenderContext::new()));
        let window_ctx = Arc::new(Mutex::new(WindowContext::new(&window)));

        let app = Arc::new(Mutex::new(Self {
            my_app: None,
            window,
            width,
            height,
            monitor,
            monitor_width,
            monitor_height,
            refresh_rate,
            start_time: Instant::now(),
            time: 0.0,
            dt: 0.0,
            frame: 0,
            input: Input::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_scroll: 0.0,
            window_ctx: window_ctx.clone(),
            render_ctx: render_ctx.clone(),
            renderer: Renderer::new(render_ctx, window_ctx),
        }));
        {
            let app_mut = &mut *app.lock().unwrap();
            app_mut.my_app = Some(A::new(app_mut as *mut _));
        }
        app
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
        {
            let buf = self.ctx().buffer("global uniform");
            self.ctx().write_buffer(buf, &uniform_data);
        }
        self.my_app().update();
    }

    fn render(&mut self) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        scope_time!("render {}", self.frame; self.frame < 4);

        self.renderer.begin_render();

        self.my_app().render();

        self.renderer.end_render(&self.window);

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
        self.window_ctx.lock().unwrap().recreate_swapchain();
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
                Event::CloseRequested => {
                    event_loop.exit();
                }
                _ => {}
            }
        }

        self.my_app().event(event);
        self.window.request_redraw();
    }

    fn my_app(&mut self) -> &mut A {
        self.my_app.as_mut().unwrap()
    }

    expose!(input.[mouse_press_x, mouse_press_y, mouse_drag_x, mouse_drag_y](m: Mouse) -> f32);
    expose!(input.[mouse_down, mouse_released, mouse_pressed](m: Mouse) -> bool);
    expose!(input.[key_down, key_released, key_pressed](k: Key) -> bool);
    expose!(input.focused() -> bool);

    pub fn ctx(&mut self) -> std::sync::MutexGuard<'_, RenderContext> {
        self.render_ctx.lock().unwrap()
    }

    pub fn write_buffer<T>(&mut self, buffer: vk::Buffer, data: &T) {
        self.ctx().write_buffer(buffer, data);
    }

    pub fn read_buffer<T>(&mut self, buffer: vk::Buffer, data: &mut T) {
        self.ctx().read_buffer(buffer, data);
    }

    pub fn center_window(&self) {
        self.window.set_outer_position(PhysicalPosition::new(
            (self.monitor_width as i32 - self.width as i32) / 2,
            (self.monitor_height as i32 - self.height as i32) / 2,
        ));
    }
}

pub struct Engine<A: App> {
    app: Option<Arc<Mutex<AppContext<A>>>>,
    window_attribs: WindowAttributes,
}

struct UnsafeEventLoop(winit::event_loop::EventLoop<()>);

impl std::ops::Deref for UnsafeEventLoop {
    type Target = winit::event_loop::EventLoop<()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for UnsafeEventLoop {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl Send for UnsafeEventLoop {}
unsafe impl Sync for UnsafeEventLoop {}

static EVENT_LOOP: LazyLock<Mutex<UnsafeEventLoop>> = LazyLock::new(|| {
    Mutex::new(UnsafeEventLoop(
        winit::event_loop::EventLoop::builder().build().unwrap(),
    ))
});

impl<T: App> Engine<T> {
    pub fn window(title: &str, width: u32, height: u32) {
        Self::with(
            WindowAttributes::default()
                .with_title(title)
                .with_inner_size(PhysicalSize::new(width, height)),
            ControlFlow::Poll,
        );
    }

    pub fn default() {
        Self::with(WindowAttributes::default(), ControlFlow::Poll);
    }

    pub fn with(window_attribs: WindowAttributes, control_flow: ControlFlow) {
        let mut engine = Self {
            app: None,
            window_attribs,
        };
        EVENT_LOOP.lock().unwrap().set_control_flow(control_flow);
        EVENT_LOOP
            .lock()
            .unwrap()
            .run_app_on_demand(&mut engine)
            .unwrap();
    }
}

impl<T: App> winit::application::ApplicationHandler for Engine<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop.primary_monitor().unwrap();
        // center window by default
        if self.window_attribs.position.is_none() {
            let PhysicalSize::<i32> { width, height } = self
                .window_attribs
                .inner_size
                .unwrap_or(winit::dpi::Size::Physical(PhysicalSize::new(800, 600)))
                .to_physical(monitor.scale_factor());
            self.window_attribs.position =
                Some(winit::dpi::Position::Physical(PhysicalPosition::new(
                    (monitor.size().width as i32 - width) / 2,
                    (monitor.size().height as i32 - height) / 2,
                )));
        }
        let window = event_loop
            .create_window(self.window_attribs.clone())
            .unwrap();
        self.app = Some(AppContext::new(window, monitor));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: Event,
    ) {
        if let Some(app) = &self.app {
            app.lock().unwrap().event(event_loop, event, window_id);
        }
    }
}
