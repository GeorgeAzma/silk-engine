use std::{
    collections::HashMap,
    ptr::{null, null_mut},
    sync::Arc,
    time::Instant,
};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop},
    window::{WindowAttributes, WindowId},
};

use crate::{
    gfx::Gfx,
    input::Input,
    prelude::ResultAny,
    util::print::{ConsoleSink, Logger, RotatingFileSink, set_global_logger},
    vulkan::{Vulkan, VulkanConfig, window::Window},
};

pub struct EngineConfig {
    pub logger: Logger,
    pub vulkan_config: VulkanConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            logger: Logger {
                sinks: vec![
                    Box::new(ConsoleSink),
                    Box::new(RotatingFileSink::new("logs/console.log", 1024 * 1024)),
                ],
            },
            vulkan_config: VulkanConfig::default(),
        }
    }
}

pub trait App {
    fn new(context: &mut Engine<Self>) -> Self
    where
        Self: Sized;
    fn update(&mut self, context: &mut Engine<Self>)
    where
        Self: Sized;
    // fn render(&mut self, gfx: &mut Gfx);
    fn on_event(
        &mut self,
        context: &mut Engine<Self>,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) where
        Self: Sized,
    {
        _ = (context, window_id, event);
    }
}

pub struct Engine<A: App> {
    pub vulkan: Arc<Vulkan>,
    event_loop: *const ActiveEventLoop,
    pub start_time: std::time::Instant,
    pub time: f32,
    pub dt: f32,
    pub fps: f32,
    pub frame: u32,
    app: *mut A,
    windows: HashMap<WindowId, Window>,
    window_input: HashMap<WindowId, Input>,
}

impl<A: App> Engine<A> {
    pub fn new(config: EngineConfig) -> ResultAny<Self> {
        std::panic::set_hook(Box::new(|panic_info| {
            let panic = |err: &str| {
                println!(
                    "\x1b[38;2;241;76;76m{err}\n\x1b[2m{}\x1b[0m",
                    crate::util::print::backtrace(2)
                );
            };
            if let Some(&str) = panic_info.payload().downcast_ref::<&str>() {
                panic(str);
            } else if let Some(str) = panic_info.payload().downcast_ref::<String>() {
                panic(str);
            } else {
                panic("")
            }
        }));

        set_global_logger(config.logger)?;

        std::fs::create_dir("res").unwrap_or_default();
        std::fs::create_dir("res/images").unwrap_or_default();
        std::fs::create_dir("res/shaders").unwrap_or_default();
        std::fs::create_dir("res/cache").unwrap_or_default();
        std::fs::create_dir("res/cache/shaders").unwrap_or_default();
        std::fs::create_dir("res/cache/vulkan").unwrap_or_default();

        let vulkan = Vulkan::new(config.vulkan_config)?;

        let mut engine = Self {
            vulkan,
            event_loop: null(),
            app: null_mut(),
            start_time: std::time::Instant::now(),
            time: 0.0,
            dt: 0.0,
            fps: 0.0,
            frame: 0,
            windows: HashMap::new(),
            window_input: HashMap::new(),
        };

        let event_loop = EventLoop::builder().build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.listen_device_events(DeviceEvents::WhenFocused);

        event_loop.run_app(&mut engine).unwrap();

        Ok(engine)
    }

    pub fn event_loop(&self) -> &ActiveEventLoop {
        unsafe { self.event_loop.as_ref() }.unwrap()
    }

    pub fn create_window(
        &mut self,
        attributes: WindowAttributes,
        gfx: &Gfx,
    ) -> ResultAny<&mut Window> {
        let window = Window::new(
            &gfx.device,
            unsafe { self.event_loop.as_ref() }.unwrap(),
            attributes,
            vec![],
            vec![],
        )?;

        let window_id = window.id();
        self.window_input.insert(window_id, Input::new());
        self.windows.insert(window_id, window);

        Ok(self.windows.get_mut(&window_id).unwrap())
    }

    pub fn window(&mut self, window_id: WindowId) -> &mut Window {
        self.windows.get_mut(&window_id).unwrap()
    }

    pub fn input(&mut self, window_id: WindowId) -> &mut Input {
        self.window_input.get_mut(&window_id).unwrap()
    }
}

impl<A: App> ApplicationHandler<()> for Engine<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.event_loop = event_loop as *const _;
        if self.app.is_null() {
            self.app = Box::leak(Box::new(A::new(self)));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.event_loop = event_loop as *const _;
        if let Some(input) = self.window_input.get_mut(&window_id) {
            let inner_size = self.windows[&window_id].inner_size();
            input.event(&event, inner_size.width, inner_size.height);
        }

        if !self.app.is_null() {
            let app: &mut A = unsafe { &mut *self.app };
            app.on_event(self, window_id, event.clone());

            if event == WindowEvent::RedrawRequested {
                let elapsed = Instant::now() - self.start_time;
                let new_time = elapsed.as_secs_f32();
                let dt = new_time - self.time;
                self.time = new_time;
                self.frame += 1;
                self.dt = dt;
                self.fps = 1.0 / self.dt;
                app.update(self);
                if let Some(input) = self.window_input.get_mut(&window_id) {
                    input.reset();
                }
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        _event: winit::event::DeviceEvent,
    ) {
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}
}
