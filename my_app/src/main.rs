use silk_engine::prelude::*;

struct App {
    window_id: WindowId,
    gfx: Gfx,
}

impl silk_engine::App for App {
    fn new(context: &mut Engine<Self>) -> Self {
        let mut gfx = Gfx::new(&context.vulkan).unwrap();

        gfx.load_img("cursor.qoi");

        let window = context
            .create_window(
                WindowAttributes::default().with_inner_size(PhysicalSize::new(1280, 720)),
                &gfx,
            )
            .unwrap();

        Self {
            window_id: window.id(),
            gfx,
        }
    }

    fn update(&mut self, ctx: &mut Engine<Self>) {
        let gfx = &mut self.gfx;

        gfx.gradient_dir = 0.0;
        gfx.rgb(255, 0, 0);
        gfx.gradient_rgb(255, 255, 0);
        gfx.rect(Pc(0.2), Pc(0.8), Pc(0.1), Pc(0.1));

        gfx.rgb(255, 255, 0);
        gfx.gradient_rgb(0, 255, 0);
        gfx.rect(Pc(0.3), Pc(0.8), Pc(0.05), Pc(0.1));

        gfx.rgb(0, 255, 0);
        gfx.gradient_rgb(0, 255, 255);
        gfx.rect(Pc(0.35), Pc(0.8), Pc(0.05), Pc(0.1));

        gfx.rgb(0, 255, 255);
        gfx.gradient_rgb(0, 0, 255);
        gfx.rect(Pc(0.4), Pc(0.8), Pc(0.1), Pc(0.1));

        gfx.stroke_color = [255, 0, 0, 255];
        gfx.stroke_width = 0.25;
        gfx.rgb(255, 255, 255);
        gfx.blur = 1.0;
        gfx.circle(Pc(0.7), Pc(0.5), Pc(0.04));

        gfx.blur = -1.0;
        gfx.circle(Pc(0.8), Pc(0.5), Pc(0.04));

        gfx.blur = 0.0;
        gfx.circle(Pc(0.7), Pc(0.4), Pc(0.04));

        gfx.blur = 1.0;
        gfx.stroke_blur = 1.0;
        gfx.circle(Pc(0.8), Pc(0.4), Pc(0.04));

        gfx.blur = -1.0;
        gfx.stroke_blur = 1.0;
        gfx.circle(Pc(0.7), Pc(0.3), Pc(0.04));

        gfx.blur = 0.0;
        gfx.stroke_blur = 1.0;
        gfx.circle(Pc(0.8), Pc(0.3), Pc(0.04));

        gfx.bold = 1.0;
        gfx.stroke_width = 0.5;
        gfx.stroke_blur = 0.0;
        gfx.blur = 0.0;
        gfx.stroke_rgb(255, 111, 32);
        gfx.rgb(32, 123, 222);
        // gfx.blur = -1.0;
        // gfx.stroke_blur = 0.5;
        gfx.font("zenmaru");
        gfx.text("鬱龍龍龜鷲鷹魁鬼鉄鬼こんにちは", Px(50), Px(110), Px(24));
        gfx.font("segoe-ui");
        gfx.text("stuff be workin", Px(150), Px(250), Px(24));
        gfx.bold = 0.0;
        gfx.stroke_width = 0.0;
        gfx.stroke_blur = 0.0;
        gfx.blur = 0.0;
        gfx.text(
            "quick brown fox jumped over a lazy dog",
            Pc(0.1),
            Px(190),
            Px(8),
        );
        gfx.rgb(255, 255, 255);
        gfx.no_gradient();
        gfx.font("zenmaru");
        gfx.text("鬱龍龍龜鷲鷹魁鬼鉄鬼こんにちは", Pc(0.7), Pc(0.7), Px(8));

        gfx.atlas();
        gfx.rect(Pc(0.4), Pc(0.4), Px(1024), Px(1024));
        gfx.no_img();

        gfx.superellipse =
            ctx.input(self.window_id).mouse_x() * 2.0 + ctx.input(self.window_id).mouse_y() + 1.0;
        gfx.stroke_width = 0.2;
        gfx.rrect(Px(730), Px(30), Px(150), Px(150), 1.0);
        gfx.rrect(Px(130), Px(30), Px(430), Px(150), 0.5);
        gfx.rrect(Px(30), Px(130), Px(60), Px(150), 0.2);

        gfx.circle(Px(30), Px(530), Px(30));

        let window = ctx.window(self.window_id);
        self.gfx.render(window);

        window.request_redraw();
    }

    fn on_event(&mut self, context: &mut Engine<Self>, window_id: WindowId, event: WindowEvent) {
        if window_id == self.window_id {
            match event {
                WindowEvent::Destroyed | WindowEvent::CloseRequested => {
                    context.event_loop().exit();
                }
                _ => {}
            }
        }
    }
}

fn main() -> ResultAny {
    let _engine = Engine::<App>::new(EngineConfig {
        vulkan_config: VulkanConfig {
            ..Default::default()
        },
        ..Default::default()
    })?;

    Ok(())
}
