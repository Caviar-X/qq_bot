use proc_qq::ClientBuilder;
use proc_qq::DeviceSource::JsonFile;
use proc_qq::{re_exports::ricq_core::protocol::version::*, *};
use qq_bot::interface::module;
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
#[tokio::main]
async fn main() {
    init_tracing_subscriber();
    let client = ClientBuilder::new()
        .authentication(Authentication::QRCode)
        .device(JsonFile("device.json".into()))
        .version(&MACOS)
        .show_rq(Some(ShowQR::OpenBySystem))
        .modules(vec![module()]).build().await.unwrap();
    client
        .start()
        .await
        .unwrap()
        .unwrap();
}

fn init_tracing_subscriber() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .without_time()
                .with_line_number(true),
        )
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_target("ricq", Level::DEBUG)
                .with_target("proc_qq", Level::DEBUG)
                .with_target("qq_bot", Level::DEBUG),
        )
        .init();
}
