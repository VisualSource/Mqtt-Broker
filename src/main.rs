use config::ConfigBuilder;

mod config;
mod core;
mod error;
mod handler;
mod packets;
mod topic_heir;
mod utils;

use crate::core::{enums::Command, App};
use crate::error::MqttError;
use crate::handler::client_handler;

use log::{debug, info};
use tokio::{select, sync::mpsc::channel};

use tokio_util::sync::CancellationToken;

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
async fn main() -> Result<(), MqttError> {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Trace)
        .init();

    let config = ConfigBuilder::new()
        .set_port(1883)
        .set_sys_interval(0)
        .build()
        .expect("Failed to start: Invalid config");
    info!("Starting MQTT Broker at: {}", config.socket_addr);

    let listener = tokio::net::TcpListener::bind(config.socket_addr)
        .await
        .map_err(MqttError::Io)?;

    let tracker = tokio_util::task::TaskTracker::new();
    let token = CancellationToken::new();
    let (tx, mut rx) = channel::<Command>(100);

    tracker.spawn(async move {
        let mut context = App::new();
        while let Some(command) = rx.recv().await {
            match command {
                Command::RegisterClient {
                    id,
                    message_channel,
                    protocol,
                    clean_session,
                    callback,
                } => {
                    context
                        .connect(id, message_channel, protocol, clean_session, callback)
                        .await
                }
                Command::Subscribe {
                    client,
                    topics,
                    callback,
                } => context.subscribe(client, topics, callback),

                Command::Publish { topic, payload } => context.publish(topic, payload).await,
                Command::Unsubscribe {
                    topics,
                    cid,
                    callback,
                } => context.unsubscribe(cid, topics, callback),
                Command::DisconnectClient(cid) => context.disconnect(cid),
                Command::Exit => break,
            }
        }

        debug!("Exiting Command loop");
    });

    loop {
        select! {
            res = listener.accept() => {
                if let Ok((stream,addr)) = res {
                    log::debug!("Connection Start: {:?}",addr);
                    let cancellation = token.clone();
                    let message_brige = tx.clone();
                    tracker.spawn(async move {
                        let (reader, writer) = tokio::io::split(stream);
                        if let Err(err) = client_handler(reader,writer,message_brige,cancellation).await {
                           log::error!("{}", err);
                        }
                        log::debug!("Exited TCP handler");
                    });
                }
            }
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    log::info!("Exiting");
    if tx.send(Command::Exit).await.is_err() {
        log::error!("Failed to exit message loop");
    }
    token.cancel();
    tracker.close();

    tracker.wait().await;

    Ok(())
}
