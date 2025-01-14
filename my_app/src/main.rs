use silk_engine::*;

pub(crate) struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
    print: Cooldown,
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
                GraphicsPipelineInfo::new()
                    .dyn_size()
                    .color_attachment(surf_format)
                    .blend_attachment_empty(),
                &[],
            );
            ctx.add_desc_set("global uniform ds", "shader", 0);
            ctx.add_buffer(
                "global uniform",
                size_of::<GlobalUniform>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            ctx.write_ds("global uniform ds", "global uniform", 0);
        }
        Self {
            app,
            print: Cooldown::ms(256),
        }
    }

    fn update(&mut self) {
        let app = &mut self.app;
        if self.print.ready() {
            println!("{:?} ({:.0} fps)", Duration::from_secs_f32(app.dt), app.fps);
            self.print.reset();
        }
        let uniform_data = GlobalUniform {
            resolution: [app.width, app.height],
            mouse_pos: [app.mouse_x, app.mouse_y],
            time: app.time,
            dt: app.dt,
        };
        app.ctx().write_buffer("global uniform", &uniform_data);
    }

    fn render(&mut self) {
        let t = (self.app.time * 3.0).sin() * 0.02;
        let gfx = self.app.gfx();
        // gfx.color = [0, 32, 55, 255];
        // for x in 0..256 {
        //     for y in 0..256 {
        //         gfx.rect(
        //             -0.99 + x as f32 / 256.0 * 1.98,
        //             -0.99 + y as f32 / 256.0 * 1.98,
        //             0.005,
        //             0.005,
        //         );
        //     }
        // }
        gfx.rotation = 0.2 + t;
        gfx.color = [255, 32, 100, 255];
        gfx.stroke_width = 0.5;
        gfx.stroke_color = [22, 132, 0, 255];
        gfx.circle(0.0, 0.4, 0.2);
        gfx.rrect_center(0.0, 0.0, 0.9, 0.3, 0.5);

        // let mut ctx = self.app.ctx();
        // ctx.bind_pipeline("pipeline");
        // ctx.bind_desc_set("global uniform ds");
        // ctx.draw(3, 1);
    }
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
