use silk_engine::*;

pub(crate) struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };
        let surf_format = app.surface_format;
        {
            let mut ctx = app.ctx();
            ctx.add_shader("shader");
            ctx.add_pipeline(
                "pipeline",
                "shader",
                GraphicsPipeline::new()
                    .dyn_size()
                    .color_attachment(surf_format)
                    .blend_attachment_empty(),
                &[],
            );
            let desc_set = ctx.add_desc_set("global uniform", "shader", 0);

            let uniform_buffer = ctx.add_buffer(
                "global uniform",
                size_of::<GlobalUniform>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            write_desc_set_uniform_buffer_whole(desc_set, uniform_buffer, 0);
        }
        Self { app }
    }

    fn update(&mut self) {
        let app = &mut self.app;
        if app.frame % 64 == 0 {
            println!(
                "{:?} ({:.0} fps)",
                Duration::from_secs_f32(app.dt),
                1.0 / app.dt
            );
        }
        let uniform_data = GlobalUniform {
            resolution: [app.width, app.height],
            mouse_pos: [app.mouse_x, app.mouse_y],
            time: app.time,
            dt: app.dt,
        };
        app.ctx().write_buffer("global uniform", &uniform_data);
        let gfx = self.app.gfx();
        for x in 0..256 {
            for y in 0..256 {
                gfx.rect(
                    -0.99 + x as f32 / 256.0 * 1.98,
                    -0.99 + y as f32 / 256.0 * 1.98,
                    0.005,
                    0.005,
                );
            }
        }
    }

    fn render(&mut self) {
        let mut ctx = self.app.ctx();
        ctx.bind_pipeline("pipeline");
        ctx.bind_desc_set("global uniform");
        ctx.draw(3, 1);
    }

    fn event(&mut self, _e: Event) {}
}

#[repr(C)]
#[derive(Default, Clone)]
pub struct GlobalUniform {
    pub resolution: [u32; 2],
    pub mouse_pos: [f32; 2],
    pub time: f32,
    pub dt: f32,
}

fn main() {
    Engine::<MyApp>::window("App", 800, 600);
}
