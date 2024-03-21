use config::ConfigBuilder;

mod config;
mod core;
mod error;
mod handler;
mod listener;
mod packets;

// Version 5 https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021
// Version 3.1.1 + Errata http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/mqtt-v3.1.1.html

// https://hivemq.github.io/mqtt-cli/docs/installation/
// C impl example
// https://codepr.github.io/posts/sol-mqtt-broker/

// Code
// https://blog.graysonhead.net/posts/rust-tcp/
// https://patshaughnessy.net/2018/3/15/how-rust-implements-tagged-unions
// https://locka99.gitbooks.io/a-guide-to-porting-c-to-rust/content/features_of_rust/types.html
// https://towardsdev.com/bitwise-operation-and-tricks-in-rust-5aea318c99b7
// https://c-for-dummies.com/blog/?p=1848
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    #[cfg(debug_assertions)]
    {
        console_subscriber::init();
    }

    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Error)
        .init();

    let config = ConfigBuilder::new()
        .set_port(1883)
        .set_sys_interval(0)
        .build()
        .expect("Failed to start: Invalid config");

    if let Err(err) = listener::listen(config).await {
        log::error!("{}", err);
    }
}
