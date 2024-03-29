pub mod app;
pub mod assets;
pub mod cooldown;
pub mod renderer;

#[tokio::main]
async fn main() {
    app::run().await;
}
