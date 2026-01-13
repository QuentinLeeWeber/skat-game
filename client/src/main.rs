use slint;

mod app;
mod conversions;
mod networking;

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    app::run().await
}
