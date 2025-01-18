use silk_engine::*;

pub struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };

        let img_data = Qoi::load("cursor");
        assert_eq!(img_data.channels, 4, "only RGBA images supported");
        let d = app.gfx().add_img("cursor", img_data.width, img_data.height);
        d.copy_from_slice(&img_data.img);

        let img_data = Qoi::load("spark");
        let d = app.gfx().add_img("spark", img_data.width, img_data.height);
        d.copy_from_slice(&img_data.img);

        Self { app }
    }

    fn update(&mut self) {}

    fn render(&mut self, gfx: &mut Renderer) {
        gfx.img("spark");
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
