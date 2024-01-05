use image::EncodableLayout;
use std::rc::Rc;
use winit::{
    event::{Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::NamedKey,
    window::Window,
    window::WindowBuilder,
};
mod renderer;
mod simple_app;

struct App {
    surface: wgpu::Surface,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Rc<Window>,
    renderer: renderer::Renderer,
    simple_app: Option<simple_app::SimpleApp>,
    start_time: std::time::Instant,
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
            size,
            window: window.clone(),
            renderer,
            simple_app: None,
            start_time: std::time::Instant::now(),
        };

        app.simple_app = Some(simple_app::SimpleApp::new(&app));

        return app;
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size = size;
        let simple_app = self.simple_app.as_mut().unwrap();
        simple_app.width = size.width;
        simple_app.width = size.height;
        if size.width > 0 && size.height > 0 {
            self.surface_config.width = size.width;
            self.surface_config.height = size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.renderer.resize(size.width, size.height);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        let simple_app = self.simple_app.as_mut().unwrap();
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                simple_app.mouse_x =
                    position.x as f32 / self.window.inner_size().width as f32 * 2.0 - 1.0;
                simple_app.mouse_y =
                    1.0 - position.y as f32 / self.window.inner_size().height as f32 * 2.0;
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => match button {
                MouseButton::Left => {
                    simple_app.mouse_left = state.is_pressed();
                }
                MouseButton::Right => {
                    simple_app.mouse_right = state.is_pressed();
                }
                MouseButton::Middle => {
                    simple_app.mouse_middle = state.is_pressed();
                }
                _ => {}
            },
            WindowEvent::Touch(touch) => {
                simple_app.mouse_x = touch.location.x as f32;
                simple_app.mouse_y = touch.location.y as f32;
                match touch.phase {
                    winit::event::TouchPhase::Started => {
                        simple_app.mouse_left = true;
                    }
                    winit::event::TouchPhase::Ended => {
                        simple_app.mouse_left = false;
                    }
                    winit::event::TouchPhase::Moved => {
                        simple_app.mouse_left = true;
                    }
                    winit::event::TouchPhase::Cancelled => {
                        simple_app.mouse_left = false;
                    }
                }
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => match event.logical_key {
                winit::keyboard::Key::Named(key) => {
                    simple_app.key[key as usize] = event.state.is_pressed();
                }
                _ => {}
            },
            _ => {}
        }
        simple_app.event(event);
        false
    }

    fn update(&mut self) {
        let now = std::time::Instant::now()
            .duration_since(self.start_time)
            .as_secs_f32();
        let simple_app = self.simple_app.as_mut().unwrap();
        simple_app.dt = now - simple_app.time;
        simple_app.time = now;
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

        output.present();

        Ok(())
    }

    pub fn create_image_2d(&self, path: &str) -> renderer::image::Image {
        renderer::image::Image::from(&self.device, &self.queue, path)
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
                            app.resize(physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            app.update();
                            match app.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => app.resize(app.size),
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
