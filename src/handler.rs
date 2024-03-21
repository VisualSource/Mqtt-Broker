use std::time::Duration;

use log::{debug, error};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt},
    select,
    sync::mpsc::{channel, Sender},
    time::Instant,
};
use tokio_util::sync::CancellationToken;

use crate::{
    core::{
        broker_info,
        enums::{ClientEvent, Command, ProtocalVersion},
    },
    error::MqttError,
    packets::{
        enums::{ConnectReturnCode, QosLevel, SubackReturnCode},
        Packet, VariableHeader,
    },
};

pub async fn client_handler<R, W>(
    read_stream: R,
    mut writer: W,
    message_bridge: Sender<Command>,
    cancellation: CancellationToken,
) -> Result<(), MqttError>
where
    W: AsyncWrite + Unpin,
    R: AsyncRead + Unpin,
{
    broker_info::client_inc();

    let mut keepalive_duration: u64 = 60;
    let mut has_connected = false;
    let mut protocol = ProtocalVersion::Unknown;
    let mut cid = None;
    let keepalive_timer = tokio::time::sleep(Duration::from_secs(60));
    let mut reader = tokio::io::BufReader::new(read_stream);
    let (tx, mut rx) = channel::<ClientEvent>(100);

    tokio::pin!(keepalive_timer);

    'ctrl: loop {
        select! {
            () = cancellation.cancelled() => {
                break 'ctrl;
            }
            () = &mut keepalive_timer => {
                break 'ctrl;
            }
            buffer = reader.fill_buf() => {
                let packet = match buffer {
                    Ok(bytes) => {
                        let len = bytes.len();
                        if len == 0 {
                            continue;
                        }

                        let (packet, packet_size) = Packet::unpack(bytes, protocol)?;

                        broker_info::sent_data(packet_size);

                        reader.consume(packet_size);
                        packet
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset || e.kind() == std::io::ErrorKind::ConnectionReset => {
                        debug!("Connection lost");
                        break 'ctrl;
                    }
                    Err(err) => {
                        error!("{}",err);
                        return Err(MqttError::Io(err));
                    }
                };

                match packet.variable {
                        VariableHeader::Connect { flags, keepalive, client_id, protocol_version, .. } => {
                            if has_connected {
                                debug!("Seen connect packet two times!");
                                //  Client can only send the CONNECT Packet once over a Network Connection.
                                // The Server MUST process a second CONNECT Packet sent from a Client as a protocol violation and disconnect the Client
                                let resp = Packet::make_connack(ConnectReturnCode::V4UnacceptableProtocal,false);

                                let len = writer.write(&resp).await?;
                                debug!("Wrote {} bytes", len);

                                break 'ctrl;
                            }

                            protocol = protocol_version;

                            let (r_tx, r_rx) = tokio::sync::oneshot::channel::<Result<(), MqttError>>();
                            cid = Some(client_id.clone());

                            if message_bridge
                            .send(Command::RegisterClient {
                                id: client_id,
                                message_channel: tx.clone(),
                                protocol,
                                clean_session: flags.clean_session(),
                                callback: r_tx,
                            }).await.is_err()
                          {
                                error!("Receiver dropped. Closing");
                                break 'ctrl;
                          }

                          r_rx.await.map_err(|_| MqttError::QueuePoisonError)??;
                          has_connected = true;
                          keepalive_duration = (keepalive as u64) + 4;
                          keepalive_timer.as_mut().reset(Instant::now() + Duration::from_secs(keepalive_duration));

                          let resp = Packet::make_connack(ConnectReturnCode::Accepted,false);

                          let len = writer.write(&resp).await?;
                          debug!("Wrote {} bytes", len);
                        },
                        VariableHeader::Subscribe { packet_id, tuples,.. } => {
                            let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;
                            let (r_tx, r_rx) =
                            tokio::sync::oneshot::channel::<Result<Vec<SubackReturnCode>, MqttError>>();

                            if message_bridge
                            .send(Command::Subscribe {
                                client: id.clone(),
                                topics: tuples,
                                callback: r_tx,
                            })
                            .await
                            .is_err()
                            {
                                error!("Receiver dropped!");
                                break 'ctrl;
                            }
                            let codes = r_rx.await.map_err(|_| MqttError::QueuePoisonError)??;

                            let resp = Packet::make_suback(packet_id, codes);

                            let len = writer.write(&resp).await?;
                            debug!("Wrote {} bytes", len);
                        },
                        VariableHeader::Unsubscribe { packet_id, tuples, .. } => {
                            let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;

                            let (r_tx, r_rx) = tokio::sync::oneshot::channel::<Result<(), MqttError>>();

                            if message_bridge
                                .send(Command::Unsubscribe {
                                    topics: tuples,
                                    client: id.clone(),
                                    callback: r_tx,
                                })
                                .await
                                .is_err()
                            {
                                error!("Receiver dropped. Closing");
                                break 'ctrl;
                            }

                            r_rx.await.map_err(|_| MqttError::QueuePoisonError)??;

                            let resp = Packet::make_unsuback(packet_id);
                            let len = writer.write(&resp).await?;
                            debug!("Wrote {} bytes", len);
                        },
                        VariableHeader::Publish { topic, packet_id, payload, .. } => {
                            message_bridge
                            .send(Command::Publish {
                                topic,
                                payload,
                            })
                            .await
                            .map_err(MqttError::ChannelError)?;

                            let data = match packet.fixed.get_qos()? {
                                QosLevel::AtMost => None,
                                QosLevel::AtLeast => {
                                    let id = packet_id.ok_or_else(|| MqttError::ProtocolViolation)?;

                                    Some(Packet::make_puback(id))
                                }
                                QosLevel::Exactly => {
                                    let id = packet_id.ok_or_else(|| MqttError::ProtocolViolation)?;

                                    Some(Packet::make_pubrec(id))
                                }
                            };

                            if let Some(resp) = data {
                                let len = writer.write(&resp).await?;
                                debug!("Wrote {} bytes", len);
                            }
                        }
                        VariableHeader::PubRec { packet_id, .. } => {
                            let resp = Packet::make_pubrel(packet_id);
                            let len = writer.write(&resp).await?;
                            debug!("Wrote {} bytes", len);
                        },
                        VariableHeader::PubRel { packet_id, .. } => {
                            let resp = Packet::make_pubcomp(packet_id);
                            let len = writer.write(&resp).await?;
                            debug!("Wrote {} bytes", len);
                        },
                        VariableHeader::PubComp { packet_id: _, ..} | VariableHeader::PubAck { packet_id: _, .. } => {}
                        VariableHeader::PingReq => {
                            keepalive_timer.as_mut().reset(Instant::now() + Duration::from_secs(keepalive_duration));
                            let resp = Packet::make_ping_resp();
                            let len = writer.write(&resp).await?;
                            debug!("Wrote {} bytes", len);
                        },
                        VariableHeader::Disconnect { .. } => {
                            debug!("Disconnect Called");
                            let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;
                            if message_bridge
                            .send(Command::DisconnectClient(id.clone()))
                            .await
                            .is_err()
                            {
                                error!("Receiver dropped!");
                                break 'ctrl;
                            }
                            break 'ctrl;
                        },
                        _ => {
                            error!("Invaild packet");
                            break 'ctrl;
                        }
                    }
            }
            event = rx.recv() => {
                if let Some(ev) = event {
                    match ev {
                        ClientEvent::Message(msg) => {
                            let len = writer.write(&msg).await?;
                            debug!("Wrote {} bytes",len);
                        },
                        ClientEvent::Disconnect => break 'ctrl,
                    }
                }

            }
        }
    }

    broker_info::client_dec();

    debug!("Client: disconnect");

    Ok(())
}
