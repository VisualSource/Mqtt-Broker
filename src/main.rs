use config::ConfigBuilder;

mod config;
mod core;
mod error;
mod handler;
mod packets;
mod topic_heir;
mod utils;

use crate::core::{broker_info, enums::Command, App};
use crate::error::MqttError;
use crate::handler::client_handler;
use crate::{
    core::enums::ClientEvent,
    packets::{enums::SubackReturnCode, Packet},
};
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
                    protocol: _,
                    clean_session,
                    callback,
                } => {
                    debug!("Register client ({})", id);
                    if context.has_client(&id) {
                        context.disconnect_client(&id).await;
                    }

                    context.update_client(&id, message_channel);

                    if clean_session {
                        context.clear_client_session(&id);
                    }

                    if callback.send(Ok(())).is_err() {
                        log::error!("Client no longer exists");
                    }
                }
                Command::Subscribe {
                    client: cid,
                    topics,
                    callback,
                } => {
                    let client = match context.get_client(&cid) {
                        None => {
                            if callback.send(Err(MqttError::Unknown)).is_err() {
                                log::error!("Client does not exist");
                            }
                            return;
                        }
                        Some(client) => (client.id, client.sender.clone()),
                    };

                    let mut codes = Vec::<SubackReturnCode>::new();
                    for (topic, qos) in topics {
                        debug!("Client {} Subscribe to {} with {}", cid, topic, qos);

                        // 4.7.2  Topics beginning with $
                        // The Server SHOULD prevent Clients from using such Topic Names to exchange messages with other Clients.
                        // Server implementations MAY use Topic Names that start with a leading $ character for other purposes.

                        let result = context.subscribe(topic, qos, client.0, client.1.clone());
                        codes.push(result);
                    }

                    if callback.send(Ok(codes)).is_err() {
                        log::error!("Client does not exist");
                    }
                }
                Command::Publish { topic, payload } => {
                    broker_info::received_published();
                    if let Ok(subs) = context.subscribers(topic.clone()) {
                        for (_, bridge, qos) in subs {
                            broker_info::sent_published();
                            let packet = Packet::make_publish(
                                false,
                                qos,
                                false,
                                topic.clone(),
                                None,
                                payload.clone(),
                            );
                            if let Err(e) = bridge.send(ClientEvent::Message(packet)).await {
                                log::error!("receiver dropped: {}", e);
                            }
                        }
                    }
                }
                Command::Unsubscribe {
                    topics,
                    cid,
                    callback,
                } => {
                    let client_id = match context.get_client(&cid) {
                        None => {
                            if callback.send(Err(MqttError::Unknown)).is_err() {
                                log::error!("Client does not exist");
                            }
                            return;
                        }
                        Some(client) => client.id,
                    };

                    for topic in topics {
                        debug!("Unsubscribe from topic '{}' for client '{}'", topic, cid);
                        if context.unsubscribe(topic.clone(), client_id).is_err() {
                            log::error!(
                                "Failed to unsubscribe from topic '{}' for user '{}'",
                                cid,
                                topic
                            );
                        }
                    }

                    if callback.send(Ok(())).is_err() {
                        log::error!("receiver dropped");
                    }
                }
                Command::DisconnectClient(cid) => {
                    debug!("Drop client: {}", cid);
                    context.remove_client(&cid);
                }
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
