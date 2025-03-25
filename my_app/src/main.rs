use silk_engine::prelude::*;

struct MyApp<'a> {
    #[allow(unused)]
    app: &'a mut AppContext<Self>,
    packer: Guillotine,
    rects: Vec<(u16, u16, u16, u16)>,
    batch: Vec<Vertex>,
    dt_accum: f32,
    max_dt: f32,
    uid: usize,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };

        let mut rects = vec![];
        let mut packer = Guillotine::new(512, 512);
        let mut area = 0;
        let mut perim = 0;
        let unpacked = (0u32..1160)
            .map(|i| {
                let range = 128;
                let rng = |off: i32| {
                    (i as i32 + off)
                        .randn_range(-range, range, 6.0)
                        .unsigned_abs() as u16
                        + 1
                };
                (rng(0), rng(i32::MAX / 2))
            })
            .collect::<Vec<_>>();
        let packed = packer.pack_all(&unpacked);
        for (i, p) in packed.iter().enumerate() {
            let (w, h) = unpacked[i];
            if let &Some((x, y)) = p {
                // if (x as usize * y as usize + i).rand() % 4 != 0 || x + y > 256 {
                //     packer.unpack(x, y, w, h);
                // } else {
                area += w as u32 * h as u32;
                rects.push((x, y, w, h));
                // }
            }
        }
        for fr in packer.free_rects.iter() {
            let (w, h) = fr.wh();
            perim += w as u32 + h as u32;
        }
        println!(
            "Pack Efficiency: {} %",
            area as f32 / (packer.width() as f32 * packer.height() as f32) * 100.0
        );
        println!(
            "Packed: {} %",
            rects.len() as f32 / unpacked.len() as f32 * 100.0
        );
        println!("Rects: {}", rects.len());
        println!("Free Rects: {}", packer.free_rects.len());
        println!("Perim Sum: {perim}");

        app.gfx.begin_batch();
        app.gfx.rgb(32, 123, 222);
        for x in 0..1000 {
            for y in 0..300 {
                app.gfx.circle(Px(x), Px(y), Px(1));
            }
        }
        let batch = app.gfx.end_batch();

        let uid = app.sfx.load("steingen").loops(4).play(&app.sfx);
        app.sfx.pause_stream();

        Self {
            app,
            packer,
            rects,
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

        gfx.begin_temp();
        gfx.stroke_width = 0.2;
        gfx.stroke_color = [32, 128, 48, 128];
        gfx.color = [64, 255, 96, 128];
        for fr in self.packer.free_rects.iter() {
            let (x, y, w, h) = fr.xywh();
            let (pw, ph) = (self.packer.width() as f32, self.packer.height() as f32);
            let (x, y) = (x as f32 / pw, y as f32 / ph);
            let (w, h) = (w as f32 / pw, h as f32 / ph);
            gfx.rect(Mn(x), Mn(y), Mn(w), Mn(h));
        }

        gfx.stroke_width = 0.2;
        gfx.stroke_color = [128, 32, 48, 128];
        gfx.color = [255, 48, 96, 128];
        for &(x, y, w, h) in self.rects.iter() {
            let (pw, ph) = (self.packer.width() as f32, self.packer.height() as f32);
            let (x, y) = (x as f32 / pw, y as f32 / ph);
            let (w, h) = (w as f32 / pw, h as f32 / ph);
            gfx.rrect(Mn(x), Mn(y), Mn(w), Mn(h), 0.4);
        }
        gfx.end_temp();

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
    }
}

fn main() {
    Engine::<MyApp>::window("App", 800, 600);
}
