use core::App;
use error::MqttError;
use server::{Event, Request};
use std::io;
use tokio::{
    io::{AsyncReadExt, Interest},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{channel, error::TryRecvError, Sender},
        RwLock,
    },
};

use crate::packets::{enums::QosLevel, Packet};
mod core;
mod error;
mod packets;
mod server;

lazy_static::lazy_static! {
    static ref APP: RwLock<App> = RwLock::new(App::new());
}

/// Version 5 https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021
/// Version 3.1.1 + Errata http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/mqtt-v3.1.1.html

/// C impl example
/// https://codepr.github.io/posts/sol-mqtt-broker/

/// Code
/// https://patshaughnessy.net/2018/3/15/how-rust-implements-tagged-unions
/// https://locka99.gitbooks.io/a-guide-to-porting-c-to-rust/content/features_of_rust/types.html
/// https://towardsdev.com/bitwise-operation-and-tricks-in-rust-5aea318c99b7
/// https://c-for-dummies.com/blog/?p=1848
#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:1883";
    if let Err(err) = listen(addr).await {
        eprintln!("{}", err);
    }
}

async fn listen(addr: &str) -> Result<(), MqttError> {
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| MqttError::Io(e))?;

    let (tx, mut rx) = channel::<Request>(100);

    tokio::spawn(async move {
        while let Some(i) = rx.recv().await {
            match i {
                Request::Publish(topic, payload) => {
                    let read = APP.read().await;
                    let subs = read.topics.get_matches(&topic);

                    for sub in subs {
                        if let Ok(packet) = Packet::make_publish(
                            false,
                            sub.1,
                            false,
                            topic.clone(),
                            None,
                            payload.clone(),
                        ) {
                            if let Err(e) = sub.0.send(Event::Message(packet)).await {
                                eprintln!("{}", e);
                            }
                        }
                    }
                }
                Request::DisconnectClient(cid) => {
                    if let Some(client) = APP.read().await.clients.get(&cid) {
                        if let Err(e) = client.sender.send(Event::Disconnect).await {
                            eprintln!("{}", e);
                        }
                    }
                }
            }
        }
    });

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let send = tx.clone();

                tokio::spawn(async move {
                    if let Err(err) = stream_handler(stream, send).await {
                        eprintln!("{}", err);
                    }
                });
            }
            Err(err) => {
                eprintln!("{}", err);
            }
        }
    }
}

async fn stream_handler(stream: TcpStream, tx: Sender<Request>) -> Result<(), MqttError> {
    println!("Connection from {}", stream.peer_addr().unwrap());

    let (stx, mut rx) = channel::<Event>(100);
    let mut cid: Option<String> = None;

    let mut reply_queue: Vec<Vec<u8>> = Vec::new();
    let mut buf: [u8; 4096];
    let mut close_con = false;
    loop {
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
            .map_err(|e| MqttError::Io(e))?;

        if ready.is_readable() {
            buf = [0; 4096];

            match stream.try_read(&mut buf) {
                Ok(0) => {
                    println!("Client disconnected");
                    break;
                }
                Ok(n) => {
                    println!("read {} bytes", n);
                    let mut result_buffer: Vec<u8> = Vec::with_capacity(n);
                    buf.take(n as u64).read_to_end(&mut result_buffer).await?;

                    match packets::Packet::unpack(&result_buffer) {
                        Ok(packet) => {
                            let data = match packet {
                                Packet::Connect(_header, body) => {
                                    let rc = packets::enums::ConnectReturnCode::Accepted;
                                    let session_present = false;

                                    if APP.read().await.clients.contains_key(&body.client_id) {
                                        // If the ClientId represents a Client already connected to the Server then the
                                        // Server MUST disconnect the existing Client.
                                        tx.send(Request::DisconnectClient(body.client_id.clone()))
                                            .await?;
                                    }

                                    {
                                        // drop write lock
                                        let mut app = APP.write().await;

                                        cid = Some(body.client_id.clone());

                                        app.add_client(body.client_id.clone(), stx.clone());

                                        // If the Server accepts a connection with CleanSession set to 0,
                                        // the value set in Session Present depends on whether the Server already has stored Session state for the supplied client ID.
                                        // If the Server has stored Session state, it MUST set Session Present to 1 in the CONNACK packet [MQTT-3.2.2-2].
                                        // If the Server does not have stored Session state, it MUST set Session Present to 0 in the CONNACK packet.
                                        // This is in addition to setting a zero return code in the CONNACK packet
                                        if body.flags.clean_session() {
                                            if let Some(client) =
                                                app.clients.get_mut(&body.client_id)
                                            {
                                                client.clear_session();
                                            }
                                        }
                                    }
                                    // If the Server accepts a connection with CleanSession set to 1,
                                    // the Server MUST set Session Present to 0 in the CONNACK packet
                                    // in addition to setting a zero return code in the CONNACK packet

                                    // If a server sends a CONNACK packet containing a non-zero return code it MUST set Session Present to 0

                                    // if do auth check client id check other checks

                                    Packet::make_connack(rc, session_present)
                                }
                                Packet::ConnAck(_, _) => todo!(),
                                Packet::Subscribe(_header, body) => {
                                    use packets::enums::SubackReturnCode;
                                    let mut codes = Vec::<SubackReturnCode>::new();
                                    let id =
                                        cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;
                                    let mut app = APP.write().await;
                                    for code in body.tuples {
                                        let result = app.add_subscriber_to_topic(
                                            code.topic,
                                            id,
                                            code.qos,
                                            stx.clone(),
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

                                    Packet::make_suback(body.packet_id, codes)
                                }
                                Packet::Unsubscribe(_header, body) => {
                                    let id =
                                        cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;
                                    {
                                        let mut app = APP.write().await;

                                        for sub in body.tuples {
                                            app.remove_subscriber_from_topic(&sub, id);
                                        }
                                    }

                                    Packet::make_unsuback(body.packet_id)
                                }
                                Packet::SubAck(_, _) => todo!(),
                                Packet::Publish(header, body) => {
                                    tx.send(Request::Publish(body.topic, body.payload))
                                        .await
                                        .map_err(|e| MqttError::ChannelError(e))?;

                                    // none on qos 0
                                    // create puback on qos 1
                                    // create pubrec on qos 2

                                    match header.get_qos()? {
                                        QosLevel::AtMostOnce => Ok(Vec::with_capacity(0)),
                                        QosLevel::AtLeastOnce => {
                                            let id = body
                                                .packet_id
                                                .ok_or_else(|| MqttError::ProtocolViolation)?;

                                            Packet::make_puback(id)
                                        }
                                        QosLevel::ExactlyOnce => {
                                            let id = body
                                                .packet_id
                                                .ok_or_else(|| MqttError::ProtocolViolation)?;

                                            Packet::make_pubrec(id)
                                        }
                                    }
                                }
                                Packet::PubAck(_, _) => {
                                    //todo!("Handle removing pending puback clients map");
                                    Ok(Vec::with_capacity(0))
                                }
                                Packet::PubRec(_, body) => Packet::make_pubrel(body.packet_id),
                                Packet::PubRel(_, body) => Packet::make_pubcomp(body.packet_id),
                                Packet::PubComp(_, _) => {
                                    //  TODO: Handle removing pending puback clients map
                                    Ok(Vec::with_capacity(0))
                                }

                                Packet::PingReq(_) => Packet::make_ping_resp(),
                                Packet::Disconnect(_) => {
                                    let mut app = APP.write().await;
                                    let id =
                                        cid.as_ref().ok_or_else(|| MqttError::FailedToGetCId)?;
                                    app.remove_client(id);
                                    break;
                                }

                                Packet::PingResp(_) | Packet::UnsubAck(_, _) => {
                                    Err(MqttError::MalformedHeader)
                                }
                            };

                            match data {
                                Ok(data) => {
                                    if data.len() > 0 {
                                        reply_queue.push(data);
                                    }
                                }
                                Err(MqttError::ClientIdentifierRejected) => {
                                    let p = Packet::make_connack(
                                        packets::enums::ConnectReturnCode::IdentifierRejected,
                                        false,
                                    )?;
                                    reply_queue.push(p);
                                    close_con = true;
                                }
                                Err(MqttError::UnsupportedProtocolVersion) => {
                                    let p = Packet::make_connack(
                                        packets::enums::ConnectReturnCode::UnacceptableProtocal,
                                        false,
                                    )?;
                                    reply_queue.push(p);
                                    close_con = true;
                                }
                                Err(err) => {
                                    eprintln!("{}", err);
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("{}", err);
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => return Err(MqttError::Io(e)),
            }
        }

        match rx.try_recv() {
            Ok(m) => match m {
                Event::Message(data) => {
                    reply_queue.push(data);
                }
                Event::Disconnect => break,
            },
            Err(TryRecvError::Empty) => {}
            Err(e) => {
                println!("Recv: {}", e);
            }
        }

        if ready.is_writable() {
            if let Some(msg) = reply_queue.pop() {
                println!("{:#?}", msg);
                match stream.try_write(&msg) {
                    Ok(n) => {
                        println!("Wrote {} bytes", n);

                        if close_con {
                            break;
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        return Err(MqttError::Io(e));
                    }
                }
            }
        }
    }

    Ok(())
}