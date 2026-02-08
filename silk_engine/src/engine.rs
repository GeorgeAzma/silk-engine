use std::{
    ops::Deref,
    sync::{Arc, Mutex},
    time::Instant,
};

use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents},
    window::WindowId,
};

use crate::{
    util::print::{ConsoleSink, Logger, RotatingFileSink, set_global_logger},
    vulkan::VulkanConfig,
};

pub type WinitEvent = winit::event::WindowEvent;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

#[derive(Event)]
pub struct WindowEvent {
    pub window_id: WindowId,
    pub window_event: WinitEvent,
}

#[derive(Resource)]
pub struct EngineConfig {
    pub logger: Logger,
    pub vulkan_config: VulkanConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            logger: Logger {
                sinks: vec![
                    Arc::new(Mutex::new(ConsoleSink)),
                    Arc::new(Mutex::new(RotatingFileSink::new(
                        "logs/console.log",
                        1024 * 1024,
                    ))),
                ],
            },
            vulkan_config: VulkanConfig::default(),
        }
    }
}

#[derive(Resource)]
pub struct Time {
    pub start_time: std::time::Instant,
    pub time: f32,
    pub dt: f32,
    pub fps: f32,
    pub frame: u32,
}

pub struct Engine;

impl Engine {
    fn setup(config: Res<EngineConfig>) {
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

        set_global_logger(config.logger.clone()).unwrap();

        use std::fs::create_dir;
        create_dir("res").unwrap_or_default();
        create_dir("res/images").unwrap_or_default();
        create_dir("res/shaders").unwrap_or_default();
        create_dir("res/cache").unwrap_or_default();
        create_dir("res/cache/shaders").unwrap_or_default();
        create_dir("res/cache/vulkan").unwrap_or_default();
    }

    fn on_event(event: On<WindowEvent>, mut time: ResMut<Time>) {
        if event.window_event == WinitEvent::RedrawRequested {
            let elapsed = Instant::now() - time.start_time;
            let new_time = elapsed.as_secs_f32();
            let dt = new_time - time.time;
            time.time = new_time;
            time.frame += 1;
            time.dt = dt;
            time.fps = 1.0 / time.dt;
        }
    }

    fn runner(app: App) -> AppExit {
        let mut context = Context::new(app);
        let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.listen_device_events(DeviceEvents::WhenFocused);
        event_loop.run_app(&mut context).unwrap();
        AppExit::Success
    }
}

impl Plugin for Engine {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time {
            start_time: Instant::now(),
            time: Default::default(),
            dt: Default::default(),
            fps: Default::default(),
            frame: Default::default(),
        })
        .add_observer(Self::on_event)
        .set_runner(Self::runner)
        .add_systems(PreStartup, Self::setup);
    }
}

#[derive(Default, Resource)]
pub struct EventLoop(*const ActiveEventLoop);
impl Deref for EventLoop {
    type Target = ActiveEventLoop;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
unsafe impl Send for EventLoop {}
unsafe impl Sync for EventLoop {}

struct Context {
    app: App,
}

impl Context {
    fn new(app: App) -> Self {
        Self { app }
    }
}

impl ApplicationHandler<()> for Context {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.app.insert_resource(EventLoop(event_loop as *const _));
        self.app.update();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        window_event: WinitEvent,
    ) {
        self.app.world_mut().trigger(WindowEvent {
            window_id,
            window_event: window_event.clone(),
        });

        self.app.update();
        if let Some(_exit) = self.app.should_exit() {
            event_loop.exit();
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
