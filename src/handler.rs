use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use bytes::{BufMut, Bytes};
use log::{error, info};
use tokio::{
    io::{AsyncWriteExt, Interest},
    net::TcpStream,
    select,
    sync::mpsc::{channel, Sender},
};

use crate::{
    core::enums::{ClientEvent, Command, ProtocalVersion},
    error::MqttError,
    packets::{
        enums::{ConnectReturnCode, QosLevel},
        Packet,
    },
    utils::queue::FifoQueue,
};

type ReplyQueue = Arc<FifoQueue<Bytes>>;

pub async fn client_handler(
    mut stream: TcpStream,
    message_handler: Sender<Command>,
) -> Result<(), MqttError> {
    info!(
        "Client starting from: {}",
        stream.peer_addr().map_err(MqttError::Io)?
    );

    // channel for receiving messages
    let (tx, mut rx) = channel::<ClientEvent>(100);

    // queue for messages that will be sent to the client
    let reply_queue: ReplyQueue = Arc::new(FifoQueue::<Bytes>::new());
    // does this client need to be close
    // https://stackoverflow.com/questions/71764138/how-to-run-multiple-tokio-async-tasks-in-a-loop-without-using-tokiospawn
    select! {
       write = async {
            info!("Starting Client write loop");

            loop {
                stream.writable().await?;
                if let Some(msg) = reply_queue.pop()? {
                    match stream.try_write(&msg) {
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
        } => { write? }
        read = async {
            info!("Starting Client read loop");
            //let client_id: ArcStr = ArcStr::new();
            // The protocal that this client is using.
            // Version 4 for MQTT 3.1.1
            // Version 5 for MQTT 5
            //let mut client_protocal: ProtocalVersion = ProtocalVersion::Unknown;
            let mut buf = [0u8;4096];
            loop {
               stream.readable().await?;

                match stream.try_read(&mut buf){
                    Ok(len) => {
                        info!("Read {} Bytes",len);
                        match Packet::unpack(&buf){
                            Ok(packet) => {
                                let result: Result<Option<Bytes>, MqttError> = match packet {
                                    Packet::Connect(_, body) => {
                                        let rc = ConnectReturnCode::Accepted;
                                        let session_present = false;

                                        let data = Packet::make_connack(rc, session_present)?;
                                        Ok(Some(data))
                                    },
                                    Packet::ConnAck(_, _) => todo!(),
                                    Packet::Subscribe(_, _) => todo!(),
                                    Packet::Unsubscribe(_, _) => todo!(),
                                    Packet::SubAck(_, _) => todo!(),
                                    Packet::Publish(header, body) => {
                                        match header.get_qos()? {
                                            QosLevel::AtMostOnce => Ok(None),
                                            QosLevel::AtLeastOnce => {
                                                let id = body
                                                    .packet_id
                                                    .ok_or_else(|| MqttError::ProtocolViolation)?;

                                                let packet = Packet::make_puback(id)?;
                                                Ok(Some(packet))
                                            }
                                            QosLevel::ExactlyOnce => {
                                                let id = body
                                                    .packet_id
                                                    .ok_or_else(|| MqttError::ProtocolViolation)?;

                                                let data = Packet::make_pubrec(id)?;
                                                Ok(Some(data))
                                            }
                                        }
                                    },
                                    Packet::PubAck(_, _) => todo!(),
                                    Packet::PubRec(_, _) => todo!(),
                                    Packet::PubRel(_, _) => todo!(),
                                    Packet::PubComp(_, _) => todo!(),
                                    Packet::UnsubAck(_, _) => todo!(),
                                    Packet::PingReq(_) => todo!(),
                                    Packet::PingResp(_) => todo!(),
                                    Packet::Disconnect(_) => todo!(),
                                };

                                match result {
                                    Ok(data) => {
                                        if let Some(message) = data {
                                            if let Err(err) = reply_queue.push(message){
                                                error!("{}",err);
                                            }
                                        }
                                    },
                                    Err(err) => {
                                        error!("{}",err);
                                    },
                                }

                            },
                            Err(err) => {
                                error!("{}",err);
                            },
                        }
                    },
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                        return  Ok::<(),MqttError>(());
                    }
                    Err(err) => {
                        error!("{}",err.kind());
                        return Err(MqttError::Io(err));
                    },
                 }
            }
        } => {
            read?
        }
        msg_loop = async {
            info!("Starting Client msg loop");
            let rp = reply_queue.clone();
            loop {
                if let Some(msg) = rx.recv().await {
                    match msg {
                        ClientEvent::Message(data) => {
                            rp.push(data)?;
                        },
                        ClientEvent::Disconnect => {
                            break;
                        },
                    }
                }
            }
            info!("Exiting msg_loop");
            Ok::<(),MqttError>(())
         } => {
            msg_loop?
         }
    }

    info!("Client disconnect");
    stream.shutdown().await.map_err(MqttError::Io)
}

#[cfg(test)]
mod tests {}
