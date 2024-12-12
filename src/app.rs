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
    app: Arc<App>,
}

impl MyApp {
    pub fn new(app: Arc<App>) -> Self {
        Self { app }
    }

    pub fn update(&mut self) {
        if self.app.frame % 256 == 0 {
            println!(
                "{:?} ({:.0} fps)",
                Duration::from_secs_f32(self.app.dt),
                1.0 / self.app.dt
            );
        }
    }

    pub fn render(&mut self) {}

    pub fn event(&mut self, _e: Event) {}
}
