use proc_qq::ClientBuilder;
use proc_qq::DeviceSource::JsonFile;
use proc_qq::{re_exports::ricq_core::protocol::version::*, *};
use qq_bot::interface::module;
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let (mut uin, mut pwd) = (0, String::new());
    for (cnt, i) in std::env::args().enumerate() {
        if i == "--uin" {
            uin = std::env::args().nth(cnt + 1).unwrap().parse().unwrap();
        } else if i == "--password" {
            pwd = std::env::args().nth(cnt + 1).unwrap();
        }
    }
    init_tracing_subscriber();
    ClientBuilder::new()
        .authentication(Authentication::UinPassword(uin, pwd))
        .device(JsonFile("device.json".into()))
        .version(&ANDROID_PHONE)
        .modules(vec![module()])
        .build()
        .await
        .unwrap()
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
                // 这里改成自己的crate名称
                .with_target("qq_bot", Level::DEBUG),
        )
        .init();
}
