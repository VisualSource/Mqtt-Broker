use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use arcstr::ArcStr;
use bytes::{BufMut, Bytes};
use log::{error, info};
use tokio::{
    io::{AsyncWriteExt, Interest},
    net::TcpStream,
    select,
    sync::mpsc::{channel, Sender},
    task::{self, JoinHandle},
};
use tokio_util::sync::CancellationToken;

use crate::{
    core::{
        enums::{ClientEvent, Command, ProtocalVersion},
        App,
    },
    error::MqttError,
    packets::{
        enums::{ConnectReturnCode, QosLevel, SubackReturnCode},
        Packet,
    },
    utils::queue::FifoQueue,
};

type ReplyQueue = Arc<FifoQueue<Bytes>>;

pub async fn client_handler(
    stream: TcpStream,
    message_handler: Sender<Command>,
    cancellation: CancellationToken,
    context: Arc<App>,
) -> Result<(), MqttError> {
    info!(
        "Client starting from: {}",
        stream.peer_addr().map_err(MqttError::Io)?
    );

    let (tx, mut rx) = channel::<ClientEvent>(100);
    let (read_stream, write_stream) = stream.into_split();
    let reply_queue: ReplyQueue = Arc::new(FifoQueue::<Bytes>::new());

    let rs_rp = reply_queue.clone();
    let read_handle = tokio::spawn(async move {
        info!("Starting reading task");
        let mut buf = [0u8; 4096];
        let mut cid: Option<String> = None;
        let mut protocal = ProtocalVersion::Unknown;
        loop {
            read_stream.readable().await?;

            match read_stream.try_read(&mut buf) {
                Ok(len) => {
                    if len == 0 {
                        continue;
                    }
                    info!("Read {} Bytes", len);
                    let result: Result<Option<Bytes>, MqttError> = match Packet::unpack(&buf) {
                        Ok(packet) => match packet {
                            Packet::Connect(_, body) => {
                                let rc = ConnectReturnCode::Accepted;
                                let session_present = false;

                                if context.has_client(&body.client_id)? {
                                    // If the ClientId represents a Client already connected to the Server then the
                                    // Server MUST disconnect the existing Client.
                                    message_handler
                                        .send(Command::DisconnectClient(body.client_id.clone()))
                                        .await
                                        .map_err(MqttError::ChannelError)?;
                                }

                                cid = Some(body.client_id.clone());

                                context.add_client(body.client_id.clone(), tx.clone())?;

                                protocal = if body.protocal_version == 4 {
                                    ProtocalVersion::Four
                                } else {
                                    ProtocalVersion::Five
                                };

                                if body.flags.clean_session() {
                                    context.clear_user_session(&body.client_id)?;
                                }

                                Ok(Some(Packet::make_connack(rc, session_present)?))
                            }
                            Packet::Subscribe(_, body) => {
                                let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;
                                let mut codes = Vec::<SubackReturnCode>::new();

                                for code in body.tuples {
                                    let result = context.add_subscriber_to_topic(
                                        code.topic,
                                        id,
                                        code.qos,
                                        tx.clone(),
                                    );

                                    if result.is_ok() {
                                        let code = match code.qos {
                                            QosLevel::AtMostOnce => {
                                                SubackReturnCode::SuccessQosZero
                                            }
                                            QosLevel::AtLeastOnce => {
                                                SubackReturnCode::SuccessQosOne
                                            }
                                            QosLevel::ExactlyOnce => {
                                                SubackReturnCode::SuccessQosTwo
                                            }
                                        };

                                        codes.push(code);

                                        continue;
                                    }

                                    codes.push(SubackReturnCode::Failure);
                                }

                                Ok(Some(Packet::make_suback(body.packet_id, codes)?))
                            }
                            Packet::Unsubscribe(_, body) => {
                                let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;
                                for sub in body.tuples {
                                    context.remove_subscriber_from_topic(&sub, id)?;
                                }

                                Ok(Some(Packet::make_unsuback(body.packet_id)?))
                            }
                            Packet::Publish(header, body) => {
                                message_handler
                                    .send(Command::Publish(body.topic.clone(), body.payload))
                                    .await
                                    .map_err(MqttError::ChannelError)?;

                                let data = match header.get_qos()? {
                                    QosLevel::AtMostOnce => None,
                                    QosLevel::AtLeastOnce => {
                                        let id = body
                                            .packet_id
                                            .ok_or_else(|| MqttError::ProtocolViolation)?;

                                        Some(Packet::make_puback(id)?)
                                    }
                                    QosLevel::ExactlyOnce => {
                                        let id = body
                                            .packet_id
                                            .ok_or_else(|| MqttError::ProtocolViolation)?;

                                        Some(Packet::make_pubrec(id)?)
                                    }
                                };

                                Ok(data)
                            }
                            Packet::PubAck(_, _) => Ok(None),
                            Packet::PubRec(_, body) => {
                                Ok(Some(Packet::make_pubrel(body.packet_id)?))
                            }
                            Packet::PubRel(_, body) => {
                                Ok(Some(Packet::make_pubcomp(body.packet_id)?))
                            }
                            Packet::PubComp(_, _) => Ok(None),

                            Packet::PingReq(_) => Ok(Some(Packet::make_ping_resp()?)),

                            Packet::Disconnect(_) => {
                                let id = cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;

                                context.remove_client(id)?;
                                return Ok(());
                            }

                            _ => unreachable!("Shoud hot have been here"),
                        },
                        Err(err) => Err(err),
                    };

                    match result {
                        Ok(data) => {
                            if let Some(message) = data {
                                rs_rp.push(message)?;
                            }
                        }
                        Err(err) => {
                            error!("{}", err);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                    return Ok::<(), MqttError>(());
                }
                Err(err) => return Err(MqttError::Io(err)),
            }
        }
    });

    let ws_rq = reply_queue.clone();
    let write_handle = tokio::spawn(async move {
        info!("Starting write task");
        loop {
            write_stream.writable().await?;
            if let Some(message) = ws_rq.pop()? {
                match write_stream.try_write(&message) {
                    Ok(len) => {
                        info!("Wrote {} bytes", len);
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
    });

    let rp_ml = reply_queue.clone();
    let ml_handle: JoinHandle<Result<(), MqttError>> = tokio::spawn(async move {
        info!("Starting message loop");
        loop {
            if let Some(message) = rx.recv().await {
                match message {
                    ClientEvent::Message(data) => {
                        rp_ml.push(data)?;
                    }
                    ClientEvent::Disconnect => return Ok(()),
                }
            }
        }
    });

    select! {
        r = read_handle => { r?? }
        w = write_handle => { w?? }
        m = ml_handle => { m?? }
        _ = cancellation.cancelled() => {
            info!("Cancelled called");
        }
    }

    info!("Client disconnect");
    Ok(())
}

#[cfg(test)]
mod tests {}
