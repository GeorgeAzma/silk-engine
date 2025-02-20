use silk_engine::prelude::*;

struct MyApp<'a> {
    #[allow(unused)]
    app: &'a mut AppContext<Self>,
    packer: Guillotine,
    rects: Vec<(u16, u16, u16, u16)>,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };

        app.gfx().add_font("segoe-ui");

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
        Self { app, packer, rects }
    }

    fn update(&mut self) {
        if self.app.frame % 256 == 0 {
            self.app
                .window
                .set_title(&format!("{:.3} ms", self.app.dt * 1000.0));
        }
    }

    fn render(&mut self, gfx: &mut Renderer) {
        // gfx.begin_temp();
        // gfx.stroke_width = 0.2;
        // gfx.stroke_color = [32, 128, 48, 128];
        // gfx.color = [64, 255, 96, 128];
        // for fr in self.packer.free_rects.iter() {
        //     let (x, y, w, h) = fr.xywh();
        //     let (pw, ph) = (self.packer.width() as f32, self.packer.height() as f32);
        //     let (x, y) = (x as f32 / pw, y as f32 / ph);
        //     let (w, h) = (w as f32 / pw, h as f32 / ph);
        //     gfx.rect(Mn(x), Mn(y), Mn(w), Mn(h));
        // }

        // gfx.stroke_width = 0.2;
        // gfx.stroke_color = [128, 32, 48, 128];
        // gfx.color = [255, 48, 96, 128];
        // for &(x, y, w, h) in self.rects.iter() {
        //     let (pw, ph) = (self.packer.width() as f32, self.packer.height() as f32);
        //     let (x, y) = (x as f32 / pw, y as f32 / ph);
        //     let (w, h) = (w as f32 / pw, h as f32 / ph);
        //     gfx.rrect(Mn(x), Mn(y), Mn(w), Mn(h), 0.4);
        // }
        // gfx.end_temp();

        gfx.bold = 0.5;
        gfx.stroke_color = [255, 0, 0, 255];
        gfx.stroke_width = 0.5;
        gfx.font("segoe-ui");
        gfx.text("‰‰‰shit and\nsuch", Px(150), Px(110), Px(64));
        gfx.no_img();
        gfx.square(Px(10), Px(10), Px(64));
        gfx.atlas();
        gfx.rect(Pc(0.5), Pc(0.5), Px(1024), Px(1024));
    }
}

fn main() {
    Engine::<MyApp>::window("App", 800, 600);
}
