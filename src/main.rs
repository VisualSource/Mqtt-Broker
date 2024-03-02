use std::borrow::BorrowMut;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};

use crate::packets::enums::QosLevel;
use crate::packets::Packet;

mod error;
mod packets;
mod server;
/// spec

/// Version 5 https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021
/// Version 3.1.1 + Errata http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/mqtt-v3.1.1.html

/// C impl example
/// https://codepr.github.io/posts/sol-mqtt-broker/

/// Code
/// https://patshaughnessy.net/2018/3/15/how-rust-implements-tagged-unions
/// https://locka99.gitbooks.io/a-guide-to-porting-c-to-rust/content/features_of_rust/types.html
/// https://towardsdev.com/bitwise-operation-and-tricks-in-rust-5aea318c99b7
/// https://c-for-dummies.com/blog/?p=1848
fn main() {
    if let Ok(listener) = TcpListener::bind("0.0.0.0:1883") {
        println!("MQTT Broker started at: 0.0.0.0:1833");
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut byte_iter = stream.borrow_mut().bytes();
                    let d = packets::Packet::unpack_stream(&mut byte_iter);

                    match d {
                        Err(err) => {
                            eprintln!("{}", err);
                        }
                        Ok(packet) => match packet {
                            Packet::Publish(header, body) => {
                                println!("PUBLISH: {:#?}", body);

                                let qos = header.get_qos().expect("Failed to get QOS");

                                let ack = match qos {
                                    QosLevel::AtMostOnce | QosLevel::AtLeastOnce => {
                                        Packet::make_puback(0)
                                    }
                                    QosLevel::ExactlyOnce => Packet::make_pubrec(0),
                                };

                                if let Ok(d) = ack.pack() {
                                    if let Err(err) = stream.write_all(d.as_slice()) {
                                        eprintln!("{}", err);
                                    }
                                }
                            }
                            Packet::Connect(header, body) => {
                                let ack = if body.flags.clean_session() {
                                    packets::Packet::make_connack(
                                        packets::enums::ConnectReturnCode::Accepted,
                                        !body.flags.clean_session(),
                                    )
                                } else {
                                    packets::Packet::make_connack(
                                        packets::enums::ConnectReturnCode::Accepted,
                                        false,
                                    )
                                };

                                if let Ok(d) = ack.pack() {
                                    if let Err(err) = stream.write_all(d.as_slice()) {
                                        eprintln!("{}", err);
                                    }
                                }
                            }
                            _ => println!("{:#?}", packet),
                        },
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        }

        drop(listener)
    }
}
