use std::time::SystemTime;

use silk_engine::*;

struct Rect(u64);

impl Rect {
    pub fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self(((w as u64) << 48) | ((h as u64) << 32) | ((x as u64) << 16) | y as u64)
    }

    pub fn xywh(&self) -> (u16, u16, u16, u16) {
        let r = self.0;
        (
            (r >> 16) as u16,
            (r >> 00) as u16,
            (r >> 48) as u16,
            (r >> 32) as u16,
        )
    }

    pub fn xy(&self) -> (u16, u16) {
        let r = self.0;
        ((r >> 16) as u16, r as u16)
    }
}

struct Packer {
    width: u16,
    height: u16,
    free_rects: Vec<Rect>,
}

impl Packer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            free_rects: vec![Rect::new(0, 0, width, height)],
        }
    }

    pub fn pack(&mut self, w: u16, h: u16) -> Option<(u16, u16)> {
        for i in 0..self.free_rects.len() {
            let (x, y, fw, fh) = self.free_rects[i].xywh();
            if w <= fw && h <= fh {
                return if w < fw && h < fh {
                    if fw - w < fh - h {
                        self.free_rects[i] = Rect::new(x + w, y, fw - w, h);
                        self.free_rects.push(Rect::new(x, y + h, fw, fh - h));
                    } else {
                        self.free_rects[i] = Rect::new(x, y + h, w, fh - h);
                        self.free_rects.push(Rect::new(x + w, y, fw - w, fh));
                    }
                    Some((x, y))
                } else if h == fh {
                    self.free_rects[i] = Rect::new(x + w, y, fw - w, fh);
                    Some((x, y))
                } else if w == fw {
                    self.free_rects[i] = Rect::new(x, y + h, fw, fh - h);
                    Some((x, y))
                } else {
                    self.free_rects.swap_remove(i);
                    Some((x, y))
                };
            }
        }
        None
    }

    pub fn reset(&mut self) {
        self.free_rects = vec![Rect::new(0, 0, self.width, self.height)];
    }

    // note: imperfect resize
    pub fn resize(&mut self, width: u16, height: u16) {
        assert!(width >= self.width && height >= self.height);
        if width == self.width && height == self.height {
            return;
        }
        let (big, small) = if width - self.width < height - self.height {
            (
                Rect::new(0, self.height, width, height - self.height),
                Rect::new(self.width, 0, width - self.width, self.height),
            )
        } else {
            (
                Rect::new(self.width, 0, width - self.width, height),
                Rect::new(0, self.height, self.width, height - self.height),
            )
        };
        self.free_rects.push(big);
        self.free_rects.push(small);
        self.width = width;
        self.height = height;
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }
}

pub struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
    packer: Packer,
    rects: Vec<Rect>,
    fill: u32,
    time: f32,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };
        Self {
            app,
            packer: Packer::new(8000, 8000),
            rects: Vec::new(),
            fill: 0,
            time: 0.0,
        }
    }

    fn update(&mut self) {
        let size = self.packer.width() as u32 * self.packer.height() as u32;
        let r = self.app.frame.rand();
        let (w, h) = ((r & 0xFF) as u16, (r >> 8) as u16 & 0xFF);
        let t = Instant::now();
        if let Some((x, y)) = self.packer.pack(w, h) {
            self.time += (Instant::now() - t).as_secs_f32() * 1000.0;
            self.rects.push(Rect::new(x, y, w, h));
            self.fill += w as u32 * h as u32;
        } else {
            let packing_efficiency = self.fill as f32 / size as f32;
            println!("Packing Efficiency: {} %", packing_efficiency * 100.0);
            println!("Pack Time: {:.2} ms", self.time);
            println!("Packed: {}", self.rects.len());
            self.packer
                .resize(self.packer.width() * 2, self.packer.height() * 2);
            std::thread::sleep(Duration::from_secs(3));
        }
    }

    fn render(&mut self, gfx: &mut Renderer) {
        gfx.color = [228, 164, 100, 155];
        gfx.stroke_color = [111, 80, 59, 155];
        gfx.stroke_width = 0.15;
        for (x, y, w, h) in self.rects.iter().map(|r| r.xywh()) {
            let (x, y, w, h) = (x as f32, y as f32, w as f32, h as f32);
            let (pw, ph) = (self.packer.width() as f32, self.packer.height() as f32);
            let (x, y, w, h) = (x / pw, y / ph, w / pw, h / ph);
            gfx.rect(Pc(x), Pc(y), Pc(w), Pc(h));
        }
        gfx.color = [32, 121, 112, 155];
        gfx.stroke_color = [22, 111, 124, 155];
        for (x, y, w, h) in self.packer.free_rects.iter().map(|r| r.xywh()) {
            let (x, y, w, h) = (x as f32, y as f32, w as f32, h as f32);
            let (pw, ph) = (self.packer.width() as f32, self.packer.height() as f32);
            let (x, y, w, h) = (x / pw, y / ph, w / pw, h / ph);
            gfx.rect(Pc(x), Pc(y), Pc(w), Pc(h));
        }
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
