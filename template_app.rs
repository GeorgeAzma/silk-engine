use silk_engine::prelude::*;

struct MyApp<'a> {
    app: &'a mut AppContext<Self>,
}

impl App for MyApp<'_> {
    fn new(app: *mut AppContext<Self>) -> Self {
        let app = unsafe { &mut *app };
        Self { app }
    }

    fn update(&mut self) {}

    fn render(&mut self, gfx: &mut Gfx) {}
}

fn main() {
    Engine::<MyApp>::window("App", 800, 600);
}
