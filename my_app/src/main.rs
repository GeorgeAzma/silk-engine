use silk_engine::*;

pub struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };

        app.gfx().load_img("cursor");
        app.gfx().load_img("spiral");

        Self { app }
    }

    fn update(&mut self) {}

    fn render(&mut self, gfx: &mut Renderer) {
        gfx.img("spiral");
        gfx.color = [255, 255, 255, 255];
        // gfx.stroke_color = [0, 255, 255, 255];
        // gfx.stroke_width = 0.3;
        gfx.rect(Pc(0.1), Pc(0.1), Pc(0.6), Pc(0.6));
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
