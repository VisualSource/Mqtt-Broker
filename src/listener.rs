use crate::config::Config;
use crate::core::{enums::Command, App};
use crate::error::MqttError;
use crate::handler::client_handler;
use log::debug;
use std::time::Duration;
use tokio::{net::TcpListener, select, sync::mpsc::channel};
use tokio_util::sync::CancellationToken;

use crate::{
    core::enums::ClientEvent,
    packets::{
        enums::{QosLevel, SubackReturnCode},
        Packet,
    },
};

pub async fn listen(config: Config) -> Result<(), MqttError> {
    let listener = TcpListener::bind(config.socket_addr)
        .await
        .map_err(MqttError::Io)?;

    let token = CancellationToken::new();
    let (tx, mut rx) = channel::<Command>(100);

    if config.sys_interval != 0 {
        let ctoken = token.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.sys_interval));
            loop {
                select! {
                    _ = interval.tick() => {
                        log::info!("Publishing borker info");
                        // publish new broker data
                    }
                    _ = ctoken.cancelled() => break
                }
            }
        });
    }

    tokio::spawn(async move {
        let mut context = App::new();
        while let Some(command) = rx.recv().await {
            match command {
                Command::RegisterClient {
                    id,
                    message_channel,
                    protocal: _,
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
                        let result = context.add_subscriber_to_topic(
                            topic,
                            &cid,
                            qos,
                            client_message_brige.clone(),
                        );

                        if result.is_ok() {
                            let code = match qos {
                                QosLevel::AtMostOnce => SubackReturnCode::SuccessQosZero,
                                QosLevel::AtLeastOnce => SubackReturnCode::SuccessQosOne,
                                QosLevel::ExactlyOnce => SubackReturnCode::SuccessQosTwo,
                            };

                            codes.push(code);
                            continue;
                        }

                        codes.push(SubackReturnCode::Failure);
                    }

                    if callback.send(Ok(codes)).is_err() {
                        log::error!("Client does not exist");
                    }
                }
                Command::Publish { topic, payload } => {
                    let targets = context.get_matches(&topic);
                    debug!("Publish topic: {}", topic);
                    for (target, qos) in targets {
                        if let Ok(packet) = Packet::make_publish(
                            false,
                            *qos,
                            false,
                            topic.clone(),
                            None,
                            payload.clone(),
                        ) {
                            if let Err(e) = target.send(ClientEvent::Message(packet)).await {
                                log::error!("receiver dropped: {}", e);
                            }
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
                    context.remove_client(&cid)
                }
                Command::Exit => break,
            }
        }
    });

    select! {
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let message_brige = tx.clone();
                        let cancellation = token.clone();
                        tokio::spawn(async move {
                            if let Err(err) = client_handler(stream,message_brige,cancellation).await {
                               log::error!("{}", err);
                            }
                        });
                    }
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            }
        } => {}
        _ = tokio::signal::ctrl_c() => {
            log::info!("Exiting");
            if tx.send(Command::Exit).await.is_err(){
                log::error!("Failed to exit message loop");
            }
            token.cancel();
        }
    }

    Ok(())
}
