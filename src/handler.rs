use std::sync::Arc;

use bytes::Bytes;
use log::{debug, error, info};
use tokio::{
    net::TcpStream,
    select,
    sync::mpsc::{channel, Sender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{
    core::enums::{ClientEvent, Command, ProtocalVersion},
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
    message_bridge: Sender<Command>,
    cancellation: CancellationToken,
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
                    debug!("Read {} Bytes with protoal: {:#?}", len, protocal);
                    let result: Result<Option<Bytes>, MqttError> = match Packet::unpack(&buf) {
                        Ok(packet) => match packet {
                            Packet::Connect(_, body) => {
                                let rc = ConnectReturnCode::Accepted;
                                let session_present = false;

                                protocal = if body.protocal_version == 4 {
                                    ProtocalVersion::Four
                                } else {
                                    ProtocalVersion::Five
                                };

                                let (r_tx, r_rx) =
                                    tokio::sync::oneshot::channel::<Result<(), MqttError>>();

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

                                Ok(Some(Packet::make_connack(rc, session_present)?))
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

                                let (r_tx, r_rx) =
                                    tokio::sync::oneshot::channel::<Result<(), MqttError>>();

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

                                if message_bridge
                                    .send(Command::DisconnectClient(id.clone()))
                                    .await
                                    .is_err()
                                {
                                    error!("Receiver dropped. Closing");
                                    return Ok(());
                                }

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
        debug!("Starting write task");
        loop {
            write_stream.writable().await?;
            if let Some(message) = ws_rq.pop()? {
                match write_stream.try_write(&message) {
                    Ok(len) => {
                        debug!("Wrote {} bytes", len);
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
