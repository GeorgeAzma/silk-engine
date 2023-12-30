#[allow(dead_code, unused_imports)]
mod app;
mod assets;
mod cooldown;

fn main() {
    pollster::block_on(app::run());
}
