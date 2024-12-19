use crate::*;

/*
Renderer:
- render graph ?? shader technique with passes

Learn and write all rendering techniques
Roadmap:
- load and render 3D model
- do simple BRDF lighting
- 3D animation


Render Graph:
- Pass(resources, targets) -> modified_targets
- pass may depend on other pass's output resource
Resource:
- image
- buffer
*/

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
