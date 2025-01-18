use silk_engine::*;

pub struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };

        app.gfx().add_img("spiral", 1024, 1024);

        Self { app }
    }

    fn update(&mut self) {}

    fn render(&mut self, gfx: &mut Renderer) {
        let d = gfx.img("spiral");
        if self.app.frame % 256 == 0 {
            println!("{}", self.app.fps);
        }
        d.iter_mut()
            .enumerate()
            .for_each(|(i, d)| *d = (i + self.app.frame as usize) as u8);
        gfx.color = [255, 255, 255, 255];
        // gfx.stroke_color = [0, 255, 255, 255];
        // gfx.stroke_width = 0.3;
        gfx.rect(Pc(0.1), Pc(0.1), Pc(0.6), Pc(0.6));
    }
}

fn main() {
    Engine::<MyApp>::window("App", 800, 600);
}
