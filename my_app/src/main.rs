use silk_engine::prelude::*;

struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
    batch: Vec<Vertex>,
    dt_accum: f32,
    max_dt: f32,
    uid: usize,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };

        app.gfx.begin_batch();
        app.gfx.font("roboto");
        app.gfx.rgb(32, 123, 222);
        for x in 0..192 {
            for y in 0..108 {
                app.gfx.text("a", Px(x), Px(y), Px(1));
            }
        }
        let batch = app.gfx.end_batch();

        let uid = app.sfx.load("steingen").loops(4).play(&app.sfx);

        Self {
            app,
            batch,
            dt_accum: 0.0,
            max_dt: 0.0,
            uid,
        }
    }

    fn update(&mut self) {
        self.dt_accum += self.app.dt;
        self.max_dt = self.max_dt.max(self.app.dt);
        if self.app.frame % 128 == 0 {
            let avg_dt = self.dt_accum / 128.0;
            self.app.window.set_title(&format!(
                "{:.2} ms  |  {:.2} ms  |  {} fps  |  {} fps",
                avg_dt * 1000.0,
                self.max_dt * 1000.0,
                (1.0 / avg_dt).round(),
                (1.0 / self.max_dt).round(),
            ));
            self.dt_accum = 0.0;
            self.max_dt = 0.0;
        }
    }

    fn render(&mut self, gfx: &mut Gfx) {
        let sfx = &self.app.sfx;
        if self.app.key_pressed(Key::Space) {
            sfx.pause(self.uid);
        }

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
        gfx.font("roboto");
        gfx.text("stuff be workin", Px(150), Px(350), Px(66));
        gfx.font("opensans");
        gfx.text("stuff be workin", Px(150), Px(290), Px(24));
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
        // gfx.atlas();
        // gfx.rect(Pc(0.3), Pc(0.3), Px(1024), Px(1024));
        gfx.batch(&self.batch);

        gfx.circle(Px(30), Px(30), Px(30));
    }
}

fn main() {
    Engine::<MyApp>::window("App", 800, 600);
}
