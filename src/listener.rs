use crate::config::Config;
use crate::core::{broker_info, enums::Command, App};
use crate::error::MqttError;
use crate::handler::client_handler;
use crate::{
    core::enums::ClientEvent,
    packets::{
        enums::{QosLevel, SubackReturnCode},
        Packet,
    },
};
use log::{debug, info};
use tokio::{select, sync::mpsc::channel};

use tokio_util::sync::CancellationToken;

pub async fn listen(config: Config) -> Result<(), MqttError> {
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
                    let client_message_brige = match context.get_client(&cid) {
                        None => {
                            if callback.send(Err(MqttError::Unknown)).is_err() {
                                log::error!("Client does not exist");
                            }
                            return;
                        }
                        Some(client) => client.sender.clone(),
                    };

                    let mut codes = Vec::<SubackReturnCode>::new();
                    for (topic, qos) in topics {
                        debug!("Client {} Subscribe to {} with {}", cid, topic, qos);

                        // 4.7.2  Topics beginning with $
                        // The Server SHOULD prevent Clients from using such Topic Names to exchange messages with other Clients.
                        // Server implementations MAY use Topic Names that start with a leading $ character for other purposes.
                        if topic.starts_with('$') && !topic.starts_with("$SYS/") {
                            codes.push(SubackReturnCode::Failure);
                            debug!("Cant not start a topic with $");
                            continue;
                        }

                        let result = context.add_subscriber_to_topic(
                            topic,
                            &cid,
                            qos,
                            client_message_brige.clone(),
                        );

                        if result.is_err() {
                            codes.push(SubackReturnCode::Failure);
                            continue;
                        }

                        let code = match qos {
                            QosLevel::AtMost => SubackReturnCode::SuccessQosZero,
                            QosLevel::AtLeast => SubackReturnCode::SuccessQosOne,
                            QosLevel::Exactly => SubackReturnCode::SuccessQosTwo,
                        };

                        codes.push(code);
                    }

                    if callback.send(Ok(codes)).is_err() {
                        log::error!("Client does not exist");
                    }
                }
                Command::Publish { topic, payload } => {
                    let targets = context.get_matches(&topic);
                    broker_info::received_published();
                    debug!("Publish topic: {}", topic);
                    for (target, qos) in targets {
                        broker_info::sent_published();

                        let packet = Packet::make_publish(
                            false,
                            *qos,
                            false,
                            topic.clone(),
                            None,
                            payload.clone(),
                        );

                        if let Err(e) = target.send(ClientEvent::Message(packet)).await {
                            log::error!("receiver dropped: {}", e);
                        }
                    }
                }
                Command::Unsubscribe {
                    topics,
                    client,
                    callback,
                } => {
                    for topic in topics {
                        debug!("Client {} Unsubscribe from {}", client, topic);
                        context.remove_subscriber_from_topic(&topic, &client);
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

    /*if config.sys_interval != 0 {
        let birge = tx.clone();
        let ctoken = token.clone();
        tracker.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.sys_interval));
            loop {
                select! {
                    _ = interval.tick() => {
                        log::info!("Publishing borker info");

                        let (bytes_received,bytes_sent,clients_connected, messages_received,messages_sent,messages_published_received,messages_published_sent) = broker_info::get_stats();

                        if birge.send(Command::Publish {
                            topic: "$SYS/broker/load/bytes/received".into(),
                            payload: Bytes::from(bytes_received.to_string())
                        }).await.is_err() {
                           break;
                        };
                        if birge.send(Command::Publish {
                            topic: "$SYS/broker/load/bytes/sent".into(),
                            payload: Bytes::from(bytes_sent.to_string())
                        }).await.is_err() {
                            break;
                        };

                        if birge.send(Command::Publish {
                            topic: "$SYS/broker/clients/connected".into(),
                            payload: Bytes::from(clients_connected.to_string())
                        }).await.is_err() {
                            break;
                        };

                        if birge.send(Command::Publish {
                            topic: "$SYS/broker/messages/received".into(),
                            payload: Bytes::from(messages_received.to_string())
                        }).await.is_err() {
                            break;
                        };

                        if birge.send(Command::Publish {
                            topic: "$SYS/broker/messages/sent".into(),
                            payload: Bytes::from(messages_sent.to_string())
                        }).await.is_err() {
                            break;
                        };

                        if birge.send(Command::Publish {
                            topic: "$SYS/broker/messages/publish/sent".into(),
                            payload: Bytes::from(messages_published_sent.to_string())
                        }).await.is_err() {
                            break;
                        };

                        if birge.send(Command::Publish {
                            topic: "$SYS/broker/messages/publish/received".into(),
                            payload: Bytes::from(messages_published_received.to_string())
                        }).await.is_err() {
                            break;
                        };
                    }
                    _ = ctoken.cancelled() => break
                }
            }

            debug!("Exiting sys info");
        });
    }*/

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
