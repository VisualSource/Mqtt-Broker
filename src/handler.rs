use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use bytes::Bytes;
use log::{debug, error, info};
use tokio::{
    io::AsyncBufReadExt,
    net::TcpStream,
    select,
    sync::mpsc::{channel, Sender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{
    core::{
        broker_info,
        enums::{ClientEvent, Command, ProtocalVersion},
        queue::FifoQueue,
    },
    error::MqttError,
    packets::{
        enums::{ConnectReturnCode, QosLevel, SubackReturnCode},
        Packet,
    },
};

pub async fn client_handler(
    stream: TcpStream,
    message_bridge: Sender<Command>,
    cancellation: CancellationToken,
) -> Result<(), MqttError> {
    info!(
        "Client starting from: {}",
        stream.peer_addr().map_err(MqttError::Io)?
    );

    let internal_cancellation = CancellationToken::new();

    broker_info::client_inc();

    let (tx, mut rx) = channel::<ClientEvent>(100);
    let (read_stream, write_stream) = stream.into_split();
    let will_disconnect = Arc::new(AtomicBool::new(false));
    let reply_queue = Arc::new(FifoQueue::<Bytes>::new());

    let rs_rp = reply_queue.clone();
    let rl_ct = internal_cancellation.clone();
    let rl_wd = will_disconnect.clone();
    let read_handle = tokio::spawn(async move {
        debug!("Starting reading task");
        let mut seen_connect_packet = false;

        let mut reader = tokio::io::BufReader::new(read_stream);
        let mut cid: Option<String> = None;
        let mut protocal = ProtocalVersion::Unknown;

        let result = select! {
            v = async {
                'loop_a: loop {
                    let (bytes, len) = match reader.fill_buf().await {
                        Ok(bytes) => {
                            let i = bytes.to_vec();
                            let len = i.len();
                            reader.consume(len);
                            (i, len)
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                            return Ok::<(), MqttError>(());
                        }
                        Err(err) => {
                            error!("{}", err);
                            return Err(MqttError::Io(err));
                        }
                    };

                    if len == 0 {
                        continue;
                    }

                    broker_info::received_data(len);

                    debug!("Read {} Bytes with protoal: {:#?}", len, protocal);

                    let result = match Packet::unpack(&bytes) {
                        Ok(packet) => match packet {
                            Packet::Connect(_, body) => {
                                if seen_connect_packet {
                                    //  Client can only send the CONNECT Packet once over a Network Connection.
                                    // The Server MUST process a second CONNECT Packet sent from a Client as a protocol violation and disconnect the Client
                                    return Err(MqttError::ProtocolViolation);
                                }

                                protocal = if body.protocal_version == 4 {
                                    ProtocalVersion::Four
                                } else {
                                    ProtocalVersion::Five
                                };

                                let (r_tx, r_rx) = tokio::sync::oneshot::channel::<Result<(), MqttError>>();

                                cid = Some(body.client_id.clone());

                                if message_bridge
                                    .send(Command::RegisterClient {
                                        id: body.client_id,
                                        message_channel: tx.clone(),
                                        protocal,
                                        clean_session: body.flags.clean_session(),
                                        callback: r_tx,
                                    })
                                    .await
                                    .is_err()
                                {
                                    error!("Receiver dropped. Closing");
                                    return Ok(());
                                }

                                r_rx.await.map_err(|_| MqttError::QueuePoisonError)??;

                                seen_connect_packet = true;

                                Ok(Some(Packet::make_connack(
                                    ConnectReturnCode::Accepted,
                                    false,
                                )?))
                            }
                            Packet::Publish(header, body) => {
                                message_bridge
                                    .send(Command::Publish {
                                        topic: body.topic,
                                        payload: body.payload,
                                    })
                                    .await
                                    .map_err(MqttError::ChannelError)?;

                                let data = match header.get_qos()? {
                                    QosLevel::AtMostOnce => None,
                                    QosLevel::AtLeastOnce => {
                                        let id =
                                            body.packet_id.ok_or_else(|| MqttError::ProtocolViolation)?;

                                        Some(Packet::make_puback(id)?)
                                    }
                                    QosLevel::ExactlyOnce => {
                                        let id =
                                            body.packet_id.ok_or_else(|| MqttError::ProtocolViolation)?;

                                        Some(Packet::make_pubrec(id)?)
                                    }
                                };

                                Ok(data)
                            }
                            Packet::Disconnect(_) => {
                                let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;

                                if message_bridge
                                    .send(Command::DisconnectClient(id.clone()))
                                    .await
                                    .is_err()
                                {
                                    error!("Receiver dropped. Closing");
                                }

                                break 'loop_a;
                            }
                            Packet::Subscribe(_, body) => {
                                let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;

                                let (r_tx, r_rx) = tokio::sync::oneshot::channel::<
                                    Result<Vec<SubackReturnCode>, MqttError>,
                                >();

                                if message_bridge
                                    .send(Command::Subscribe {
                                        client: id.clone(),
                                        topics: body.tuples,
                                        callback: r_tx,
                                    })
                                    .await
                                    .is_err()
                                {
                                    error!("Receiver dropped!");
                                    return Ok(());
                                }

                                let codes =
                                    r_rx.await.map_err(|_| MqttError::QueuePoisonError)??;

                                Ok(Some(Packet::make_suback(body.packet_id, codes)?))
                            }
                            Packet::Unsubscribe(_, body) => {
                                let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;

                                let (r_tx, r_rx) = tokio::sync::oneshot::channel::<Result<(), MqttError>>();

                                if message_bridge
                                    .send(Command::Unsubscribe {
                                        topics: body.tuples,
                                        client: id.clone(),
                                        callback: r_tx,
                                    })
                                    .await
                                    .is_err()
                                {
                                    error!("Receiver dropped. Closing");
                                    return Ok(());
                                }

                                r_rx.await.map_err(|_| MqttError::QueuePoisonError)??;

                                Ok(Some(Packet::make_unsuback(body.packet_id)?))
                            }
                            Packet::PubComp(_, _) | Packet::PubAck(_, _) => Ok(None),
                            Packet::PubRec(_, body) => Ok(Some(Packet::make_pubrel(body.packet_id)?)),
                            Packet::PubRel(_, body) => Ok(Some(Packet::make_pubcomp(body.packet_id)?)),

                            Packet::PingReq(_) => Ok(Some(Packet::make_ping_resp()?)),

                            _ => unreachable!("Shoud hot have been here"),
                        },
                        Err(err) => Err(err),
                    };

                    match result {
                        Ok(data) => {
                            if let Some(message) = data {
                                if rl_wd.load(std::sync::atomic::Ordering::Relaxed) {
                                    continue;
                                }
                                rs_rp.push(message)?;
                            }
                        }
                        Err(MqttError::UnacceptableProtocolLevel) => {
                            rs_rp.push(Packet::make_connack(
                                ConnectReturnCode::V4UnacceptableProtocal,
                                false,
                            )?)?;

                            rl_wd.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(err) => {
                            error!("{}", err);
                            return Err(err);
                        }
                    }
                }

                Ok(())
            } => v,
            _ = rl_ct.cancelled() => Ok(())
        };

        debug!("Exiting read loop");

        rl_ct.cancel();

        result
    });

    let wl_rq = reply_queue.clone();
    let wl_wd = will_disconnect.clone();
    let wl_ct = internal_cancellation.clone();
    let write_handle = tokio::spawn(async move {
        debug!("Starting write task");

        let result: Result<(), MqttError> = select! {
            v = async {
                loop {
                    write_stream.writable().await?;
                    if wl_wd.load(std::sync::atomic::Ordering::Relaxed) && wl_rq.is_empty()? {
                        debug!("Client is disconnecting, and write queue is empty. Exiting.");
                        return Ok(());
                    }
                    if let Some(message) = wl_rq.pop()? {
                        match write_stream.try_write(&message) {
                            Ok(len) => {
                                broker_info::sent_data(len);
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            Err(e) => {
                                return Err(MqttError::Io(e));
                            }
                        }
                    }
                }
            } => {
                v
            }
            _ = wl_ct.cancelled() => Ok(())
        };

        /*'wl: loop {
            select! {
                v = write_stream.writable() => {
                    v?;
                    debug!("Write");

                }
                _ = wl_ct.cancelled() => break 'wl
            }
        }*/

        wl_ct.cancel();

        debug!("Exiting Write loop");

        result
    });

    let ml_rp = reply_queue.clone();
    let ml_wd = will_disconnect.clone();
    let ml_ct = internal_cancellation.clone();
    let ml_handle: JoinHandle<Result<(), MqttError>> = tokio::spawn(async move {
        debug!("Starting message loop");

        let result: Result<(), MqttError> = select! {
            v = async {
                'message: loop {
                    if let Some(message) = rx.recv().await {
                        match message {
                            ClientEvent::Message(data) => {
                                if ml_wd.load(std::sync::atomic::Ordering::Relaxed) {
                                    // client is disconneting ignoring new messages
                                    continue;
                                }
                                ml_rp.push(data)?;
                            }
                            ClientEvent::Disconnect => {
                                debug!("Message Loop: Force close client");
                                break 'message;
                            }
                        }
                    }
                }

                Ok(())
            } => v,
            _ = ml_ct.cancelled() => Ok(())
        };

        debug!("Exiting Message loop");
        ml_ct.cancel();
        result
    });

    select! {
        _ = internal_cancellation.cancelled() => {
            info!("Internal closed");
        }
        _ = cancellation.cancelled() => {
            internal_cancellation.cancel();
            info!("Cancelled called");
        }
    }
    debug!("Waiting for message loop to exit");
    let _ = ml_handle.await?;
    debug!("Message loop has exited.");

    debug!("Waiting for write handle to exit");
    let _ = write_handle.await?;
    debug!("write handle has exited.");

    debug!("Waiting for read handle to exit");
    let _ = read_handle.await?;
    debug!("Read handle has exited.");

    debug!("Clean up finished");

    info!("Client disconnect");

    broker_info::client_dec();

    Ok(())
}

#[cfg(test)]
mod tests {}
