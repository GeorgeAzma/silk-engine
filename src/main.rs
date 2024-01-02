#[allow(dead_code, unused_imports)]
mod app;
mod assets;
mod cooldown;

#[tokio::main]
async fn main() {
    app::run().await;
}
