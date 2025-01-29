#![feature(
    mapped_lock_guards,
    once_cell_get_mut,
    slice_as_chunks,
    slice_as_array,
    str_from_raw_parts
)]
use std::any::TypeId;
pub use std::{
    collections::{HashMap, HashSet},
    fs,
    process::abort,
    ptr::{self, null, null_mut},
    rc::Rc,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    monitor::MonitorHandle,
    window::Window,
    {event_loop::ControlFlow, window::WindowId},
    {platform::run_on_demand::EventLoopExtRunOnDemand, window::WindowAttributes},
};

mod input;
mod print;
mod qoi;
use input::*;
pub use input::{Key, Mouse};
pub use print::*;
pub use qoi::Qoi;
mod gfx;
pub use gfx::*;
mod util;
pub use util::*;
mod buddy_alloc;
mod contain_range;
mod event;
pub use event::*;
mod rand;
pub use rand::*;

#[cfg(not(test))]
pub const RES_PATH: &str = "res";
#[cfg(test)]
pub const RES_PATH: &str = "../target/test_res";

pub static INIT_PATHS: LazyLock<()> = LazyLock::new(|| {
    fs::create_dir_all(RES_PATH).unwrap_or_default();
    fs::create_dir_all(format!("{RES_PATH}/shaders")).unwrap_or_default();
    fs::create_dir_all(format!("{RES_PATH}/images")).unwrap_or_default();
    fs::create_dir_all(format!("{RES_PATH}/fonts")).unwrap_or_default();
    #[cfg(not(debug_assertions))]
    fs::create_dir_all(format!("{RES_PATH}/cache/shaders")).unwrap_or_default();
});

pub trait App: Sized {
    fn new(app: *mut AppContext<Self>) -> Self;
    fn update(&mut self);
    fn render(&mut self, gfx: &mut Renderer);
    fn event(&mut self, _e: WindowEvent) {}
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
    pub fps: f32,
    pub frame: u32,
    input: Input,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_scroll: f32,
    pub surface_format: vk::Format,
    ctx: Arc<Mutex<RenderCtx>>,
    renderer: Renderer,
    dispatchers: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl<A: App> AppContext<A> {
    pub fn new(window: Window, monitor: MonitorHandle) -> Arc<Mutex<Self>> {
        scope_time!("init");
        *INIT_PATHS;
        let PhysicalSize {
            width: monitor_width,
            height: monitor_height,
        } = monitor.size();
        let PhysicalSize { width, height } = window.inner_size();
        let refresh_rate =
            (monitor.refresh_rate_millihertz().unwrap_or(60) as f32 / 1000.0).round() as u32;
        log!(
            "Monitor: {} {monitor_width}x{monitor_height} {refresh_rate}hz",
            monitor.name().unwrap_or_default(),
        );

        let ctx = Arc::new(Mutex::new(RenderCtx::new(&window)));
        let surf_fmt = ctx.lock().unwrap().surface_format.format;
        {
            let mut ctx = ctx.lock().unwrap();
            ctx.add_shader("fxaa");
            ctx.add_pipeline(
                "fxaa",
                "fxaa",
                GraphicsPipelineInfo::default()
                    .blend_attachment_empty()
                    .dyn_size()
                    .color_attachment(surf_fmt)
                    .topology(vk::PrimitiveTopology::TRIANGLE_STRIP),
                &[],
            );
            ctx.add_desc_set("fxaa ds", "fxaa", 0);
            ctx.write_ds_sampler("fxaa ds", "linear", 1);
        }
        let app = Arc::new(Mutex::new(Self {
            my_app: None,
            window,
            width: 0,
            height: 0,
            monitor,
            monitor_width,
            monitor_height,
            refresh_rate,
            start_time: Instant::now(),
            time: 0.0,
            dt: 0.0,
            fps: 0.0,
            frame: 0,
            input: Input::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_scroll: 0.0,
            ctx: ctx.clone(),
            surface_format: surf_fmt,
            renderer: Renderer::new(ctx.clone()),
            dispatchers: Default::default(),
        }));
        {
            let app_ptr = &*app.lock().unwrap() as *const AppContext<A>;
            let app_mut = unsafe { app_ptr.cast_mut().as_mut().unwrap() };
            app_mut.my_app = Some(A::new(app_ptr as *mut _));
            app_mut.dispatcher().post(&WindowResize::new(width, height));
        }
        app
    }

    fn update(&mut self) {
        scope_time!("update {}", self.frame; self.frame < 4);
        let now = Instant::now().duration_since(self.start_time).as_secs_f32();
        self.dt = now - self.time;
        self.fps = 1.0 / self.dt;
        self.time = now;
        self.my_app().update();
    }

    fn render(&mut self) {
        if self.width != 0 && self.height != 0 {
            scope_time!("render {}", self.frame; self.frame < 4);

            self.ctx().wait_prev_frame();

            self.my_app.as_mut().unwrap().render(&mut self.renderer);
            self.renderer.flush();

            let optimal_size = self.ctx().begin_frame();
            self.resize(optimal_size.width, optimal_size.height);

            // make sure rendered_img is ready to be written in fs color output
            self.ctx().set_img_layout(
                "rendered image",
                ImgLayout::COLOR,
                vk::PipelineStageFlags2::TOP_OF_PIPE,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            );

            // Render (write rendered_img color output at fs shader)
            let (width, height) = (self.width, self.height);
            self.ctx().begin_render(
                width,
                height,
                "rendered image view",
                if MSAA > 1 {
                    "sampled rendered image view"
                } else {
                    ""
                },
            );
            self.renderer.render();
            self.ctx().end_render();

            // make sure rendered_img color output is written to read in fxaa fs shader
            self.ctx().set_img_layout(
                "rendered image",
                ImgLayout::SHADER_READ,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags2::FRAGMENT_SHADER,
                vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                vk::AccessFlags2::SHADER_READ,
            );

            // make sure fxaa_img is ready to be written in fs color output
            self.ctx().set_img_layout(
                "fxaa image",
                ImgLayout::COLOR,
                vk::PipelineStageFlags2::TOP_OF_PIPE,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            );

            // FXAA
            self.ctx()
                .begin_render(width, height, "fxaa image view", "");
            self.ctx().bind_pipeline("fxaa");
            self.ctx().bind_ds("fxaa ds");
            self.ctx().draw(3, 1);
            self.ctx().end_render();

            // make sure fxaa_img color output is written
            self.ctx().set_img_layout(
                "fxaa image",
                ImgLayout::SRC,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags2::BLIT,
                vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                vk::AccessFlags2::TRANSFER_READ,
            );

            // make sure swap_img is ready to be blitted to
            let swap_img = self.ctx().cur_img();
            self.ctx().set_img_layout(
                &swap_img,
                ImgLayout::DST,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags2::BLIT,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::TRANSFER_WRITE,
            );

            // blit fxaa_img into swap_img for presenting
            self.ctx().blit("fxaa image", &swap_img);

            // make sure swap_img is ready for presenting
            self.ctx().set_img_layout(
                &swap_img,
                ImgLayout::PRESENT,
                vk::PipelineStageFlags2::BLIT,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::AccessFlags2::TRANSFER_WRITE,
                vk::AccessFlags2::NONE,
            );

            let optimal_size = self.ctx.lock().unwrap().end_frame(&self.window);
            self.resize(optimal_size.width, optimal_size.height);
        }
        self.renderer.reset();

        self.input.reset();
        self.frame += 1;
    }

    fn resize(&mut self, mut width: u32, mut height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        let optimal_size = self.ctx.lock().unwrap().recreate_swapchain();
        width = optimal_size.width;
        height = optimal_size.height;
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        let e = WindowResize::new(width, height);
        self.renderer.on_resize(&e);
        self.dispatcher().post(&e);
        if width != 0 && height != 0 {
            let mut ctx = self.ctx.lock().unwrap();
            // resize rendered image
            queue_idle();
            ctx.try_remove_img("rendered image");
            ctx.add_img(
                "rendered image",
                &ImageInfo::new()
                    .width(width)
                    .height(height)
                    .format(self.surface_format)
                    .usage(ImgUsage::COLOR | ImgUsage::SAMPLED),
                MemProp::GPU,
            );
            ctx.add_img_view("rendered image view", "rendered image");

            if MSAA > 1 {
                ctx.try_remove_img("sampled rendered image");
                ctx.add_img(
                    "sampled rendered image",
                    &ImageInfo::new()
                        .width(width)
                        .height(height)
                        .samples(MSAA)
                        .format(self.surface_format)
                        .usage(ImgUsage::COLOR | ImgUsage::TRANSIENT),
                    MemProp::GPU,
                );
                ctx.add_img_view("sampled rendered image view", "sampled rendered image");
            }

            // rewrite rendered ds image
            ctx.write_ds_img("fxaa ds", "rendered image view", ImgLayout::SHADER_READ, 0);
            // resize fxaa image
            ctx.try_remove_img("fxaa image");
            ctx.add_img(
                "fxaa image",
                &ImageInfo::new()
                    .width(width)
                    .height(height)
                    .format(self.surface_format)
                    .usage(ImgUsage::COLOR | ImgUsage::SRC),
                MemProp::GPU,
            );
            ctx.add_img_view("fxaa image view", "fxaa image");
        }
        self.resize(optimal_size.width, optimal_size.height);
    }

    fn event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent, window_id: WindowId) {
        if window_id == self.window.id() {
            self.input.event(&event, self.width, self.height);
            self.mouse_x = self.input.mouse_x();
            self.mouse_y = self.input.mouse_y();
            self.mouse_scroll = self.input.mouse_scroll();
            match &event {
                WindowEvent::Resized(size) => {
                    self.resize(size.width, size.height);
                }
                WindowEvent::RedrawRequested => {
                    self.update();
                    self.render();
                }
                WindowEvent::Focused(focused) => {
                    if !*focused {
                        self.input.reset();
                    }
                }
                WindowEvent::Destroyed | WindowEvent::CloseRequested => {
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

    pub fn gfx(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    pub fn ctx(&mut self) -> std::sync::MutexGuard<'_, RenderCtx> {
        self.ctx.lock().unwrap()
    }

    pub fn center_window(&self) {
        self.window.set_outer_position(PhysicalPosition::new(
            (self.monitor_width as i32 - self.width as i32) / 2,
            (self.monitor_height as i32 - self.height as i32) / 2,
        ));
    }

    fn dispatcher<T: Event + 'static>(&mut self) -> &mut Dispatcher<T> {
        let tid = TypeId::of::<T>();
        self.dispatchers
            .entry(tid)
            .or_insert_with(|| Box::new(Dispatcher::<T>::new()))
            .downcast_mut()
            .unwrap()
    }

    pub fn sub<T: Event + 'static>(&mut self, f: fn(&T)) {
        self.dispatcher().sub(f);
    }

    pub fn unsub<T: Event + 'static>(&mut self, f: fn(&T)) {
        self.dispatcher().unsub(f);
    }

    pub fn sub_method<T: Event + 'static, U>(&mut self, slf: &U, f: fn(&U, &T)) {
        self.dispatcher().sub_method(slf, f);
    }

    pub fn sub_method_mut<T: Event + 'static, U>(&mut self, slf: &mut U, f: fn(&mut U, &T)) {
        self.dispatcher().sub_method_mut(slf, f);
    }

    pub fn unsub_method<T: Event + 'static, U, V>(&mut self, slf: &U, f: fn(V, &T)) {
        self.dispatcher().unsub_method(slf, f);
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

static PANIC_HOOK: LazyLock<()> = LazyLock::new(|| {
    std::panic::set_hook(Box::new(|panic_info| {
        let panic = |s: &str| {
            println!(
                "panicked: \x1b[38;2;241;76;76m{}\x1b[0m\n\x1b[2m{}\x1b[0m",
                s,
                crate::backtrace(1)
            );
        };
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            panic(s);
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            panic(s);
        } else {
            panic("")
        }
    }));
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
        *PANIC_HOOK;
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
        event: WindowEvent,
    ) {
        if let Some(app) = &self.app {
            app.lock().unwrap().event(event_loop, event, window_id);
        }
    }
}
