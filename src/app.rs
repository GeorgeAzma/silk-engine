use crate::renderer;
use std::rc::Rc;
use winit::{
    event::{Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
    window::Window,
    window::WindowBuilder,
};
mod simple_app;

struct App {
    surface: wgpu::Surface,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    surface_config: wgpu::SurfaceConfiguration,
    width: u32,
    height: u32,
    window: Rc<Window>,
    renderer: renderer::Renderer,
    simple_app: Option<simple_app::SimpleApp>,
    start_time: std::time::Instant,
    time: f32,
    dt: f32,
    mouse: [bool; 5],
    mouse_pressed: [bool; 5],
    mouse_x: f32,
    mouse_y: f32,
    mouse_scroll: f32,
    mouse_press_x: [f32; 5],
    mouse_press_y: [f32; 5],
    key: [bool; 194],
    key_pressed: [bool; 194],
}

impl App {
    async fn new(window: Rc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();
        let device = Rc::new(device);
        let queue = Rc::new(queue);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let renderer = renderer::Renderer::new(&device, &queue, &surface_config);

        let mut app = Self {
            surface,
            device,
            queue,
            surface_config,
            width: size.width,
            height: size.height,
            window: window.clone(),
            renderer,
            simple_app: None,
            start_time: std::time::Instant::now(),
            time: 0.0,
            dt: 0.0,
            mouse: [false; 5],
            mouse_pressed: [false; 5],
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_scroll: 0.0,
            mouse_press_x: [0.0; 5],
            mouse_press_y: [0.0; 5],
            key: [false; 194],
            key_pressed: [false; 194],
        };

        app.simple_app = Some(simple_app::SimpleApp::new(&app));

        return app;
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.renderer.resize(width, height);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse_x =
                    position.x as f32 / self.window.inner_size().width as f32 * 2.0 - 1.0;
                self.mouse_y =
                    1.0 - position.y as f32 / self.window.inner_size().height as f32 * 2.0;
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                self.mouse[Self::mouse_button_idx(*button)] = state.is_pressed();
                if state.is_pressed() {
                    self.mouse_press_x[Self::mouse_button_idx(*button)] = self.mouse_x;
                    self.mouse_press_y[Self::mouse_button_idx(*button)] = self.mouse_y;
                }
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                use winit::event::MouseScrollDelta;
                match delta {
                    MouseScrollDelta::LineDelta(_, y) => self.mouse_scroll = *y,
                    MouseScrollDelta::PixelDelta(_) => todo!(),
                }
            }
            WindowEvent::Touch(touch) => {
                self.mouse_x = touch.location.x as f32;
                self.mouse_y = touch.location.y as f32;
                match touch.phase {
                    winit::event::TouchPhase::Started => self.mouse[0] = true,
                    winit::event::TouchPhase::Ended => self.mouse[0] = false,
                    winit::event::TouchPhase::Moved => self.mouse[0] = true,
                    winit::event::TouchPhase::Cancelled => self.mouse[0] = false,
                }
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key) => {
                    self.key[key as usize] = event.state.is_pressed();
                }
                _ => {}
            },
            _ => {}
        }
        let simple_app = self.simple_app.as_mut().unwrap();
        simple_app.event(event);
        false
    }

    fn update(&mut self) {
        let now = std::time::Instant::now()
            .duration_since(self.start_time)
            .as_secs_f32();
        self.dt = now - self.time;
        self.time = now;
        let ptr = std::ptr::addr_of_mut!(*self);
        let simple_app = self.simple_app.as_mut().unwrap();
        simple_app.app = ptr;
        simple_app.update();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let simple_app = self.simple_app.as_mut().unwrap();
        simple_app.render(&mut self.renderer);
        self.renderer.render(&mut encoder, &view);

        self.queue.submit(std::iter::once(encoder.finish()));

        if self.width != 0 && self.height != 0 {
            output.present();
        }

        self.mouse_scroll = 0.0;
        self.mouse_pressed = self.mouse.clone();
        self.key_pressed = self.key.clone();

        Ok(())
    }

    #[allow(dead_code)]
    pub fn create_image_2d(&self, path: &str) -> renderer::image::Image {
        renderer::image::Image::from(&self.device, &self.queue, path)
    }

    fn mouse_button_idx(mouse_button: MouseButton) -> usize {
        match mouse_button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Back => 3,
            MouseButton::Forward => 4,
            MouseButton::Other(..) => 0,
        }
    }

    #[allow(dead_code)]
    pub fn mouse_pressed(&self, m: MouseButton) -> bool {
        return !self.mouse_pressed[Self::mouse_button_idx(m)]
            && self.mouse[Self::mouse_button_idx(m)];
    }

    #[allow(dead_code)]
    pub fn mouse_released(&self, m: MouseButton) -> bool {
        self.mouse_pressed[Self::mouse_button_idx(m)] && !self.mouse[Self::mouse_button_idx(m)]
    }

    #[allow(dead_code)]
    pub fn mouse_down(&self, m: MouseButton) -> bool {
        self.mouse[Self::mouse_button_idx(m)]
    }

    #[allow(dead_code)]
    pub fn mouse_press_x(&self, m: MouseButton) -> f32 {
        self.mouse_press_x[Self::mouse_button_idx(m)]
    }

    #[allow(dead_code)]
    pub fn mouse_press_y(&self, m: MouseButton) -> f32 {
        self.mouse_press_y[Self::mouse_button_idx(m)]
    }

    #[allow(dead_code)]
    pub fn mouse_drag_x(&self, m: MouseButton) -> f32 {
        self.mouse_x - self.mouse_press_x[Self::mouse_button_idx(m)]
    }

    #[allow(dead_code)]
    pub fn mouse_drag_y(&self, m: MouseButton) -> f32 {
        self.mouse_y - self.mouse_press_y[Self::mouse_button_idx(m)]
    }

    #[allow(dead_code)]
    pub fn key_pressed(&self, k: KeyCode) -> bool {
        !self.key_pressed[k as usize] && self.key[k as usize]
    }

    #[allow(dead_code)]
    pub fn key_released(&self, k: KeyCode) -> bool {
        self.key_pressed[k as usize] && !self.key[k as usize]
    }

    #[allow(dead_code)]
    pub fn key_down(&self, k: KeyCode) -> bool {
        self.key[k as usize]
    }
}

pub async fn run() {
    let size = winit::dpi::PhysicalSize::new(500, 500);
    let event_loop = EventLoop::new().unwrap();
    let window = std::rc::Rc::new(
        WindowBuilder::new()
            .with_inner_size(size)
            .build(&event_loop)
            .unwrap(),
    );

    let mut app = App::new(window).await;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, window_id } if window_id == app.window.id() => {
                if !app.input(&event) {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(physical_size) => {
                            app.resize(physical_size.width, physical_size.height);
                        }
                        WindowEvent::RedrawRequested => {
                            app.update();
                            match app.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => app.resize(app.width, app.height),
                                Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        _ => (),
                    }
                }
            }
            Event::AboutToWait => {
                app.window.request_redraw();
            }
            _ => (),
        })
        .unwrap();
}
