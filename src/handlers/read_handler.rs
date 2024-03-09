use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use log::{debug, error};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead},
    select,
    sync::mpsc::Sender,
    time::Instant,
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

pub async fn handle_read_stream<Reader>(
    message_bridge: Sender<Command>,
    token: CancellationToken,
    queue: Arc<FifoQueue<Bytes>>,
    tx: Sender<ClientEvent>,
    reader: Reader,
) -> Result<(), MqttError>
where
    Reader: AsyncRead + Unpin,
{
    let mut keepalive_duration: u64 = 60;
    let mut protocal = ProtocalVersion::Unknown;
    let mut seen_connect_packet = false;
    let mut cid: Option<String> = None;
    let keepalive = tokio::time::sleep(Duration::from_secs(60));
    let mut reader = tokio::io::BufReader::new(reader);

    tokio::pin!(keepalive);

    loop {
        select! {
            buffer = reader.fill_buf() => {
                let payload = match buffer {
                    Ok(bytes) => {
                        let len = bytes.len();
                        if len == 0 {
                            continue;
                        }

                        debug!("Read {} bytes",len);

                        let buffer_data = bytes.to_vec();
                        reader.consume(len);
                        buffer_data
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                        break;
                    }
                    Err(err) => {
                        error!("{}", err);
                        return Err(MqttError::Io(err));
                    }
                };

                let result = match Packet::unpack(&payload) {
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
                            keepalive_duration = body.keepalive as u64;
                            keepalive.as_mut().reset(Instant::now() + Duration::from_secs(keepalive_duration));

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
                                    let id = body.packet_id.ok_or_else(|| MqttError::ProtocolViolation)?;

                                    Some(Packet::make_puback(id)?)
                                }
                                QosLevel::ExactlyOnce => {
                                    let id = body.packet_id.ok_or_else(|| MqttError::ProtocolViolation)?;

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

                            debug!("Exiting client");

                            break;
                        }
                        Packet::Subscribe(_, body) => {
                            let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;

                            let (r_tx, r_rx) =
                                tokio::sync::oneshot::channel::<Result<Vec<SubackReturnCode>, MqttError>>();

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

                            let codes = r_rx.await.map_err(|_| MqttError::QueuePoisonError)??;

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
                        Packet::PingReq(_) => {
                            debug!("renew keepalive");
                            keepalive.as_mut().reset(Instant::now() + Duration::from_secs(keepalive_duration));
                            Ok(Some(Packet::make_ping_resp()?))
                        }
                        _ => unreachable!("Shoud hot have been here"),
                    },
                    Err(err) => Err(err),
                };

                match result {
                    Ok(data) => {
                        if let Some(message) = data {
                            queue.push(message)?;
                        }
                    }
                    Err(MqttError::ProtocolViolation) | Err(MqttError::MalformedHeader) => {
                        break;
                    }
                    Err(MqttError::UnacceptableProtocolLevel) => {
                        queue.push(Packet::make_connack(
                            ConnectReturnCode::V4UnacceptableProtocal,
                            false,
                        )?)?;

                        break;
                    }
                    Err(err) => {
                        error!("{}", err);
                        return Err(err);
                    }
                }
            }
            () = &mut keepalive => {
                break;
            }
            () = token.cancelled() => {
                break;
            }
        }
    }

    token.cancel();

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    fn init() {
        let _ = env_logger::builder()
            .filter(None, log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_read_steam_error() {
        init();
        let _reader = tokio_test::io::Builder::new()
            .read_error(std::io::Error::from(ErrorKind::BrokenPipe))
            .build();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_read_stream() {
        init();
        let _reader = tokio_test::io::Builder::new()
            .read(b"test")
            .read(b"test3")
            .build();
    }
}
