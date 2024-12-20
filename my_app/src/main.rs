use silk_engine::*;

pub(crate) struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };
        Self { app }
    }

    fn update(&mut self) {
        if self.app.frame % 512 == 0 {
            println!(
                "{:?} ({:.0} fps)",
                Duration::from_secs_f32(self.app.dt),
                1.0 / self.app.dt
            );
        }
        if self.app.frame > 8 {
            // abort();
        }
    }

    fn render(&mut self) {
        let mut ctx = self.app.ctx();
        ctx.bind_pipeline("pipeline");
        ctx.bind_desc_set("global uniform");
        ctx.draw(3, 1);
    }

    fn event(&mut self, _e: Event) {}
}

fn main() {
    Engine::<MyApp>::window("App", 800, 600);
}
