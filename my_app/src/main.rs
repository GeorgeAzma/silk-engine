use silk_engine::*;

pub struct MyApp {
    app: Arc<AppContext<Self>>,
}

impl App for MyApp {
    fn new(app: Arc<AppContext<Self>>) -> Self {
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

    fn render(&mut self) {}

    fn event(&mut self, _e: Event) {}
}

fn main() {
    Engine::<MyApp>::new();
    Engine::<MyApp>::new();
}
