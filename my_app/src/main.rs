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

        let _font = Font::new("segoe-ui", 64);
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

    fn update(&mut self) {}

    fn render(&mut self, gfx: &mut Renderer) {
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

        gfx.color = [255, 255, 255, 255];
        // gfx.rrectc(Pc(0.5), Pc(0.5), Pc(0.3), Pc(0.1), 1.0);
        // gfx.area(Pc(0.4), Pc(0.1), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 11);
        // gfx.area(Pc(0.6), Pc(0.1), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 12);
        // gfx.area(Pc(0.8), Pc(0.1), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 13);

        // gfx.area(Pc(0.2), Pc(0.5), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 14);
        // gfx.area(Pc(0.4), Pc(0.5), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 15);
        // gfx.area(Pc(0.6), Pc(0.5), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 16);
        // gfx.area(Pc(0.8), Pc(0.5), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 17);

        // gfx.area(Pc(0.2), Pc(0.8), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 18);
        // gfx.area(Pc(0.4), Pc(0.8), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 19);
        // gfx.area(Pc(0.6), Pc(0.8), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 20);
        // gfx.area(Pc(0.8), Pc(0.8), Pc(0.3), Pc(0.3));
        // self.font.draw(gfx, 21);
    }
}

fn main() {
    Engine::<MyApp>::window("App", 800, 600);
}
