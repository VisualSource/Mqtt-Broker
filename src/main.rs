use core::{enums::Command, App};
use error::MqttError;
use handler::client_handler;
use std::{sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    select,
    sync::{mpsc::channel, RwLock},
};
use tokio_util::sync::CancellationToken;

use crate::{core::enums::ClientEvent, packets::Packet};

mod config;
mod core;
mod error;
mod handler;
mod packets;
mod server;
mod utils;

lazy_static::lazy_static! {
    static ref APP: RwLock<App> = RwLock::new(App::new());
}

// Version 5 https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021
// Version 3.1.1 + Errata http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/mqtt-v3.1.1.html

// C impl example
// https://codepr.github.io/posts/sol-mqtt-broker/

// Code
// https://patshaughnessy.net/2018/3/15/how-rust-implements-tagged-unions
// https://locka99.gitbooks.io/a-guide-to-porting-c-to-rust/content/features_of_rust/types.html
// https://towardsdev.com/bitwise-operation-and-tricks-in-rust-5aea318c99b7
// https://c-for-dummies.com/blog/?p=1848
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();
    let addr = "0.0.0.0:1883";
    if let Err(err) = listen(addr).await {
        eprintln!("{}", err);
    }
}

async fn listen(addr: &str) -> Result<(), MqttError> {
    let listener = TcpListener::bind(addr).await.map_err(MqttError::Io)?;

    let state = Arc::new(App::new());
    let token = CancellationToken::new();
    let (tx, mut rx) = channel::<Command>(100);

    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;

            // publish new broker data
        }
    });

    let ctx_m = state.clone();
    tokio::spawn(async move {
        while let Some(i) = rx.recv().await {
            match i {
                Command::Publish(topic, payload) => {
                    /*if let Ok(topics) = ctx_m.topics.read() {
                        let subs = topics.get_matches(&topic);
                        for sub in subs {
                            if let Ok(packet) = Packet::make_publish(
                                false,
                                sub.1,
                                false,
                                topic.clone(),
                                None,
                                payload.clone(),
                            ) {
                                if let Err(e) = sub.0.send(ClientEvent::Message(packet)).await {
                                    log::error!("{}", e);
                                }
                            }
                        }
                    }*/
                }
                Command::DisconnectClient(cid) => {
                    /*if let Err(err) = ctx_m.disconnect_client(&cid).await {
                        log::error!("{}", err);
                    }*/
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
                        let context = state.clone();
                        tokio::spawn(async move {
                            if let Err(err) = client_handler(stream,message_brige,cancellation, context).await {
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
            log::info!("Exit");
            token.cancel();
        }
    }

    Ok(())
}
/*
async fn stream_handler(stream: TcpStream, tx: Sender<Request>) -> Result<(), MqttError> {
    CLIENTS_CONNECTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            .map_err(MqttError::Io)?;

        if ready.is_readable() {
            buf = [0; 4096];

            match stream.try_read(&mut buf) {
                Ok(0) => {
                    println!("Client disconnected");
                    break;
                }
                Ok(n) => {
                    BYTES_RECEIVED.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
                    MESSAGES_RECEIVED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
                                    MESSAGES_PUBLISH_RECEIVED
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    tx.send(Request::Publish(body.topic, body.payload))
                                        .await
                                        .map_err(MqttError::ChannelError)?;

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
                                    if !data.is_empty() {
                                        reply_queue.push(data);
                                    }
                                }
                                Err(MqttError::ClientIdentifierRejected) => {
                                    let p = Packet::make_connack(
                                        packets::enums::ConnectReturnCode::V4IdentifierRejected,
                                        false,
                                    )?;
                                    reply_queue.push(p);
                                    close_con = true;
                                }
                                Err(MqttError::UnsupportedProtocolVersion) => {
                                    let p = Packet::make_connack(
                                        packets::enums::ConnectReturnCode::V4UnacceptableProtocal,
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
                match stream.try_write(&msg) {
                    Ok(n) => {
                        println!("Wrote {} bytes", n);
                        MESSAGES_SENT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        BYTES_SENT.fetch_add(n, std::sync::atomic::Ordering::Relaxed);

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
*/
