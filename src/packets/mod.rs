use std::mem::size_of;

use bytes::{BufMut, Bytes, BytesMut};

use crate::{
    core::enums::ProtocalVersion,
    error::MqttError,
    packets::{enums::PacketType, headers::fixed_header::FixedHeader},
};

use self::{
    enums::{ConnectReturnCode, QosLevel, SubackReturnCode},
    headers::{connack::AcknowledgeFlags, connect::Flags},
    traits::FromBytes,
    utils::{encode_length, unpack_bytes, unpack_properties, unpack_string, unpack_u16},
};

pub mod enums;
mod headers;
mod traits;
mod utils;

#[derive(Debug)]
pub enum VariableHeader {
    Connect {
        flags: Flags,
        /// The Keep Alive is a Two Byte Integer which is a time interval measured in seconds. It is the maximum time interval that is permitted to elapse between the point at which the Client finishes transmitting one MQTT Control Packet and the point it starts sending the next.
        /// It is the responsibility of the Client to ensure that the interval between MQTT Control Packets being sent does not exceed the Keep Alive value.
        keepalive: u16,

        client_id: String,
        username: Option<String>,
        password: Option<String>,
        will_topic: Option<String>,
        will_message: Option<String>,
        protocol_version: ProtocalVersion,

        session_expiry_interval: Option<u32>,
        receive_maximum: u16,
        maximum_packet_size: Option<u32>,
        topic_alias_maximum: u16,
        request_response_info: bool,
        request_problem_info: bool,
        user_properties: Vec<(String, String)>,
        auth_method: Option<String>,
        auth_data: Option<Bytes>,
    },
    ConnAck {
        acknowledge_flags: AcknowledgeFlags,
        return_code: ConnectReturnCode,
    },
    Subscribe {
        packet_id: u16,
        tuples: Vec<(String, QosLevel)>,
    },
    Unsubscribe {
        packet_id: u16,
        tuples: Vec<String>,
    },
    Publish {
        topic: String,
        packet_id: Option<u16>,
        payload: Bytes,
    },
    SubAck {
        packet_id: u16,
        return_codes: Vec<SubackReturnCode>,
    },
    PubAck {
        packet_id: u16,
    },
    PubRec {
        packet_id: u16,
    },
    PubRel {
        packet_id: u16,
    },
    PubComp {
        packet_id: u16,
    },
    UnsubAck {
        packet_id: u16,
    },
    PingReq,
    PingResp,
    Disconnect,
}

impl VariableHeader {
    fn pack(self) -> Bytes {
        let mut bytes = BytesMut::new();

        match self {
            VariableHeader::Connect {
                flags,
                keepalive,
                client_id,
                username,
                password,
                will_topic,
                will_message,
                protocol_version,
                session_expiry_interval,
                receive_maximum,
                maximum_packet_size,
                topic_alias_maximum,
                request_response_info,
                request_problem_info,
                user_properties,
                auth_method,
                auth_data,
            } => {
                let will = flags.will();
                let has_psd = flags.has_password();
                let has_usr = flags.has_username();

                bytes.put_u16(4); // str len
                bytes.put_slice(b"MQTT");
                bytes.put_u8(protocol_version.into()); // protocal version
                bytes.put_u8(flags.into());
                bytes.put_u16(keepalive);
                bytes.put_u16(client_id.len() as u16);
                bytes.put(client_id.as_bytes());

                if will {
                    if let Some(wt) = will_topic {
                        bytes.put_u16(wt.len() as u16);
                        bytes.put(wt.as_bytes());
                    }
                    if let Some(wm) = will_message {
                        bytes.put_u16(wm.len() as u16);
                        bytes.put(wm.as_bytes());
                    }
                }

                if has_usr {
                    if let Some(usr) = username {
                        bytes.put_u16(usr.len() as u16);
                        bytes.put(usr.as_bytes());
                    }
                }

                if has_psd {
                    if let Some(psd) = password {
                        bytes.put_u16(psd.len() as u16);
                        bytes.put(psd.as_bytes());
                    }
                }
            }
            VariableHeader::ConnAck {
                acknowledge_flags,
                return_code,
            } => {
                bytes.put_u8(acknowledge_flags.into());
                bytes.put_u8(return_code.into());
            }
            VariableHeader::Subscribe { packet_id, tuples } => {
                bytes.put_u16(packet_id);
                for (topic, qos) in tuples {
                    bytes.put_u16(topic.len() as u16);
                    bytes.put(topic.as_bytes());
                    bytes.put_u8(qos.into());
                }
            }
            VariableHeader::Unsubscribe { packet_id, tuples } => {
                bytes.put_u16(packet_id);

                for x in tuples {
                    bytes.put_u16(x.len() as u16);
                    bytes.put(x.as_bytes());
                }
            }
            VariableHeader::Publish {
                topic,
                packet_id,
                payload,
            } => {
                bytes.put_u16(topic.len() as u16);
                bytes.put(topic.as_bytes());

                if let Some(id) = packet_id {
                    bytes.put_u16(id);
                }

                bytes.put(payload);
            }
            VariableHeader::SubAck {
                packet_id,
                return_codes,
            } => {
                bytes.put_u16(packet_id);
                for code in return_codes {
                    bytes.put_u8(code.into());
                }
            }
            VariableHeader::UnsubAck { packet_id }
            | VariableHeader::PubComp { packet_id }
            | VariableHeader::PubRel { packet_id }
            | VariableHeader::PubRec { packet_id }
            | VariableHeader::PubAck { packet_id } => {
                bytes.put_u16(packet_id);
            }

            VariableHeader::Disconnect | VariableHeader::PingReq | VariableHeader::PingResp => {}
        }

        bytes.freeze()
    }
    fn unpack<'a, I>(iter: &mut I, fixed: &FixedHeader) -> Result<Self, MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        match fixed.get_packet_type()? {
            PacketType::Connect => {
                let protocal_name = unpack_string(iter)?;
                if &protocal_name != "MQTT" {
                    return Err(MqttError::UnknownProtocol);
                }

                let protocol_version = ProtocalVersion::from(
                    *iter
                        .next()
                        .ok_or_else(|| MqttError::RequiredByteMissing("Missing protocal byte"))?,
                );

                if protocol_version != ProtocalVersion::Four
                /*|| connect_header.protocal_version != 5*/
                {
                    return Err(MqttError::UnacceptableProtocolLevel);
                }

                let flags = Flags::from(
                    iter.next()
                        .ok_or_else(|| MqttError::RequiredByteMissing("Missing connect flags"))?,
                );

                if flags.validate_flags() {
                    return Err(MqttError::MalformedHeader);
                }

                let keepalive = unpack_u16(iter)?;

                if protocol_version == ProtocalVersion::Five {
                    let props = unpack_properties(iter)?;
                }

                let client_id = {
                    let id = unpack_string(iter)?;

                    if id.is_empty() {
                        if !flags.clean_session() {
                            return Err(MqttError::ClientIdentifierRejected);
                        }
                        uuid::Uuid::new_v4().to_string()
                    } else {
                        id
                    }
                };

                let (will_topic, will_message) = if flags.will() {
                    if protocol_version == ProtocalVersion::Five {
                        todo!("Implement Variable string header");

                        // Will Delay interval
                        // Payload Format undicator
                        // Message Expiry Interval
                        // Content Type
                        // Response Topic
                        // Correlation Data
                        // User Property
                    }

                    (Some(unpack_string(iter)?), Some(unpack_string(iter)?))
                } else {
                    (None, None)
                };

                let username = if flags.has_username() {
                    Some(unpack_string(iter)?)
                } else {
                    None
                };

                let password = if flags.has_password() {
                    Some(unpack_string(iter)?)
                } else {
                    None
                };

                Ok(Self::Connect {
                    flags,
                    keepalive,
                    client_id,
                    username,
                    password,
                    will_topic,
                    will_message,
                    protocol_version,
                    session_expiry_interval: None,
                    receive_maximum: 0,
                    maximum_packet_size: None,
                    topic_alias_maximum: 0,
                    request_response_info: true,
                    request_problem_info: true,
                    user_properties: Vec::default(),
                    auth_method: None,
                    auth_data: None,
                })
            }
            PacketType::Connack => {
                let flags_byte = iter.next().ok_or_else(|| MqttError::MissingByte)?;
                let flags = AcknowledgeFlags::from(*flags_byte);

                let rc_byte = iter.next().ok_or_else(|| MqttError::MissingByte)?;
                let rc = ConnectReturnCode::try_from(rc_byte)?;

                Ok(Self::ConnAck {
                    acknowledge_flags: flags,
                    return_code: rc,
                })
            }
            PacketType::Publish => {
                let topic = unpack_string(iter)?;

                /*
                 * Message len is calculated subtracting the length of the variable header
                 * from the Remaining Length field that is in the Fixed Header
                 */
                let mut len = fixed.get_remaing_len();

                let packet_id = if fixed.get_qos()? > QosLevel::AtMost {
                    let id = Some(unpack_u16(iter)?);
                    len -= size_of::<u16>();
                    id
                } else {
                    None
                };

                len -= size_of::<u16>() + topic.len();

                let payload = unpack_bytes(iter, len)?;

                Ok(Self::Publish {
                    topic,
                    packet_id,
                    payload,
                })
            }
            PacketType::Puback => {
                let id = unpack_u16(iter)?;
                Ok(Self::PubAck { packet_id: id })
            }
            PacketType::Pubrec => {
                let id = unpack_u16(iter)?;
                Ok(Self::PubRec { packet_id: id })
            }
            PacketType::Pubrel => {
                let id = unpack_u16(iter)?;
                Ok(Self::PubRel { packet_id: id })
            }
            PacketType::Pubcomp => {
                let id = unpack_u16(iter)?;
                Ok(Self::PubComp { packet_id: id })
            }
            PacketType::Subscribe => {
                if !fixed.get_dup() && fixed.get_qos()? != QosLevel::AtLeast && fixed.get_retain() {
                    return Err(MqttError::MalformedHeader);
                }
                let mut len = fixed.get_remaing_len();

                // # Variable header

                let packet_id = unpack_u16(iter)?;
                len -= size_of::<u16>();

                // # Payload
                /*
                 * Read in a loop all remaining bytes specified by len of the Fixed Header.
                 * From now on the payload consists of 3-tuples formed by:
                 *  - topic filter (string)
                 *  - qos
                 */
                let mut tuples = Vec::new();
                while len > 0 {
                    let topic = unpack_string(iter)?;
                    len -= topic.len() + size_of::<u16>();

                    let qos = QosLevel::try_from(
                        *iter.next().ok_or_else(|| MqttError::MalformedHeader)?,
                    )?;

                    len -= size_of::<u8>();

                    tuples.push((topic, qos));
                }

                if tuples.is_empty() {
                    return Err(MqttError::ProtocolViolation);
                }

                Ok(Self::Subscribe { packet_id, tuples })
            }
            PacketType::Suback => {
                let packet_id = unpack_u16(iter)?;

                let mut len = fixed.get_remaing_len() - size_of::<u16>();

                let mut return_codes = Vec::new();
                while len > 0 {
                    let byte = iter.next().ok_or_else(|| MqttError::MissingByte)?;

                    return_codes.push(SubackReturnCode::try_from(byte)?);

                    len -= size_of::<u8>()
                }

                Ok(Self::SubAck {
                    packet_id,
                    return_codes,
                })
            }
            PacketType::Unsubscribe => {
                let mut len = fixed.get_remaing_len();
                let mut tuples = Vec::<String>::new();
                let packet_id = unpack_u16(iter)?;
                len -= size_of::<u16>();

                while len > 0 {
                    len -= size_of::<u16>();

                    let topic = unpack_string(iter)?;
                    len -= topic.len();

                    tuples.push(topic);
                }

                Ok(Self::Unsubscribe { packet_id, tuples })
            }
            PacketType::Unsuback => {
                let id = unpack_u16(iter)?;
                Ok(Self::UnsubAck { packet_id: id })
            }
            PacketType::PingReq => Ok(Self::PingReq),
            PacketType::PingResp => Ok(Self::PingResp),
            PacketType::Disconnect => Ok(Self::Disconnect),
            PacketType::Auth => todo!(),
        }
    }
}

#[derive(Debug)]
pub struct Packet {
    pub fixed: FixedHeader,
    pub variable: VariableHeader,
}

impl Packet {
    pub fn new(header: FixedHeader, variable: VariableHeader) -> Self {
        Self {
            fixed: header,
            variable,
        }
    }
    pub fn make_publish(
        dup: bool,
        qos: QosLevel,
        retain: bool,
        topic: String,
        packet_id: Option<u16>,
        payload: Bytes,
    ) -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::Publish, dup, qos, retain, 0),
            variable: VariableHeader::Publish {
                topic,
                packet_id,
                payload,
            },
        }
        .pack()
    }
    pub fn make_pubcomp(packet_id: u16) -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::Pubcomp, false, QosLevel::AtMost, false, 0),
            variable: VariableHeader::PubComp { packet_id },
        }
        .pack()
    }
    pub fn make_pubrel(packet_id: u16) -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::Pubrel, false, QosLevel::AtLeast, false, 0),
            variable: VariableHeader::PubRel { packet_id },
        }
        .pack()
    }
    pub fn make_pubrec(packet_id: u16) -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::Pubrec, false, QosLevel::AtMost, false, 0),
            variable: VariableHeader::PubRec { packet_id },
        }
        .pack()
    }
    pub fn make_puback(packet_id: u16) -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::Puback, false, QosLevel::AtMost, false, 0),
            variable: VariableHeader::PubAck { packet_id },
        }
        .pack()
    }
    pub fn make_unsuback(packet_id: u16) -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::Unsuback, false, QosLevel::AtMost, false, 0),
            variable: VariableHeader::UnsubAck { packet_id },
        }
        .pack()
    }
    pub fn make_ping_resp() -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::PingResp, false, QosLevel::AtMost, false, 0),
            variable: VariableHeader::PingResp,
        }
        .pack()
    }
    pub fn make_suback(packet_id: u16, rc: Vec<SubackReturnCode>) -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::Suback, false, QosLevel::AtMost, false, 0),
            variable: VariableHeader::SubAck {
                packet_id,
                return_codes: rc,
            },
        }
        .pack()
    }
    pub fn make_connack(rc: ConnectReturnCode, session_present: bool) -> Bytes {
        Self {
            fixed: FixedHeader::new(PacketType::Connack, false, QosLevel::AtMost, false, 0),
            variable: VariableHeader::ConnAck {
                acknowledge_flags: AcknowledgeFlags::new(session_present),
                return_code: rc,
            },
        }
        .pack()
    }

    pub fn pack(self) -> Bytes {
        let mut buffer = BytesMut::new();

        self.fixed.as_byte(&mut buffer);

        let variable = self.variable.pack();

        encode_length(variable.len(), &mut buffer);

        buffer.put(variable);

        buffer.freeze()
    }
    pub fn unpack(bytes: &[u8]) -> Result<Self, MqttError> {
        let mut iter = bytes.iter();

        let fixed = FixedHeader::from_bytes(&mut iter, None)?;
        let variable = VariableHeader::unpack(&mut iter, &fixed)?;
        Ok(Self { fixed, variable })
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::{core::enums::ProtocalVersion, packets::enums::QosLevel};

    use super::{headers::fixed_header::FixedHeader, Packet, VariableHeader};
    // https://cedalo.com/blog/mqtt-packet-guide/
    #[test]
    fn test_unpack_connect_packet() {
        let mut data = vec![
            0x10, // Fixed Header
            0x1e, // Length
            0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, // MQTT
            0x04, // version
            0xc2, // Connect Flags
            0x00, 0x3c, // keepalive (60)
            0x00, 0x04, 0x6d, 0x79, 0x50, 0x79, // Client Id with length (4, "myPy")
            0x00, 0x06, 0x63, 0x6c, 0x69, 0x65, 0x6e,
            0x74, // Username with length (6,"client")
            0x00, 0x04, 0x70, 0x61, 0x73, 0x73, // Password with length (4,"pass")
        ];

        let packet = Packet::unpack(&mut data).expect("Failed to parse connect packet");

        if let VariableHeader::Connect {
            flags,
            keepalive,
            username,
            password,
            client_id,
            ..
        } = packet.variable
        {
            assert_eq!(packet.fixed.get_remaing_len(), 30);

            assert_eq!(keepalive, 60);

            assert!(flags.clean_session());
            assert!(flags.has_password());
            assert!(flags.has_username());
            assert!(!flags.will());
            assert_eq!(flags.will_qos().expect("QOS"), QosLevel::AtMost);
            assert!(!flags.will_retain());

            assert!(username.is_some());
            assert!(password.is_some());

            let usr = username.expect("Failed to get user");
            let psd = password.expect("Failed to get password");

            assert_eq!(usr.len(), 6, "Body Username is not of length '4'");
            assert_eq!(psd.len(), 4, "Body Password is not of length '4'");
            assert_eq!(&client_id, "myPy");
            assert_eq!(&psd, "pass");
            assert_eq!(&usr, "client");

            assert_eq!(
                flags.will_qos().expect("Failed to get qos"),
                QosLevel::AtMost
            );

            assert_eq!(keepalive, 60)
        } else {
            panic!("Packet was not a connect packet");
        }
    }

    #[test]
    fn test_unpack_publish_packet() {
        let data = vec![
            0x33, // Fixed Header QOS 1, Retain 1
            0x0E, // Length 14
            0x00, 0x04, 0x69, 0x6e, 0x66, 0x6f, // topic "info"
            0x00, 0x02, // packet_id: 2
            0x43, 0x65, 0x64, 0x61, 0x6c, 0x6f, // Message "Cedalo"
        ];

        let packet = Packet::unpack(&data).expect("Failed to parse connect packet");

        if let VariableHeader::Publish {
            packet_id,
            payload,
            topic,
        } = packet.variable
        {
            assert_eq!(packet.fixed.get_remaing_len(), 14);

            assert_eq!(
                packet.fixed.get_qos().expect("Failed to get QOS"),
                QosLevel::AtLeast
            );
            assert!(packet.fixed.get_retain());

            assert_eq!(&topic, "info");

            assert!(packet_id.is_some());
            assert_eq!(packet_id.expect("Failed to get packet id"), 2);

            assert_eq!(payload.len(), 6);

            let s = String::from_utf8(payload.to_vec()).expect("Failed to parse string");

            assert_eq!(&s, "Cedalo");
        } else {
            panic!("Invalid packet type");
        }
    }

    #[test]
    fn test_unpack_subscribe_packet() {
        let data = vec![
            0x82, // Header
            0x0C, // Len
            0x00, 0x01, // pkt id
            // Start Tubles
            0x00, 0x07, 0x6d, 0x79, 0x74, 0x6f, 0x70, 0x69, 0x63, // String "mytopic"
            0x01, // Qos
        ];

        let packet = Packet::unpack(&data).expect("Failed to parse connect packet");

        if let VariableHeader::Subscribe { packet_id, tuples } = packet.variable {
            assert_eq!(packet.fixed.get_remaing_len(), 12);

            assert_eq!(packet_id, 1);

            assert_eq!(tuples.len(), 1);
            assert_eq!(tuples[0].0, "mytopic");
            assert_eq!(tuples[0].1, QosLevel::AtLeast);
        } else {
            panic!("Invalid packet");
        }
    }

    #[test]
    fn test_unpack_unsubscribe_packet() {
        let data = vec![
            0xA2, // Fixed Header
            0x08, // length
            0x00, 0x01, // pkt_id = 1
            0x00, 0x04, 0x69, 0x6e, 0x66, 0x6f, // string "info"
        ];

        let packet = Packet::unpack(&data).expect("Failed to parse connect packet");

        if let VariableHeader::Unsubscribe { packet_id, tuples } = packet.variable {
            assert_eq!(packet.fixed.get_remaing_len(), 8, "remaing packet length");

            assert_eq!(packet_id, 1);

            assert_eq!(tuples.len(), 1);
            assert_eq!(tuples[0], "info");
        } else {
            panic!("Invalid packet");
        }
    }

    #[test]
    fn test_pack_connect_packet() {
        let header = FixedHeader::new(
            super::enums::PacketType::Connect,
            false,
            QosLevel::AtMost,
            false,
            30,
        );

        let flags = crate::packets::headers::connect::Flags::new(
            true,
            false,
            QosLevel::AtMost,
            false,
            true,
            true,
        );

        let variable = VariableHeader::Connect {
            flags,
            keepalive: 60,
            client_id: "myPy".into(),
            username: Some("client".into()),
            password: Some("pass".into()),
            will_topic: None,
            will_message: None,
            protocol_version: ProtocalVersion::Four,
            session_expiry_interval: None,
            receive_maximum: 0,
            maximum_packet_size: None,
            topic_alias_maximum: 0,
            request_response_info: false,
            request_problem_info: false,
            user_properties: Vec::default(),
            auth_method: None,
            auth_data: None,
        };

        let packet = Packet::new(header, variable);

        let bytes = packet.pack();

        let data: [u8; 32] = [
            0x10, // Fixed Header
            0x1e, // Length
            0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, // MQTT
            0x04, // version
            0xc2, // Connect Flags
            0x00, 0x3c, // keepalive (60)
            0x00, 0x04, 0x6d, 0x79, 0x50, 0x79, // Client Id with length (4, "myPy")
            0x00, 0x06, 0x63, 0x6c, 0x69, 0x65, 0x6e,
            0x74, // Username with length (6,"client")
            0x00, 0x04, 0x70, 0x61, 0x73, 0x73, // Password with length (4,"pass")
        ];

        assert_eq!(data.to_vec(), bytes)
    }

    #[test]
    fn test_pack_publish_packet() {
        let payload: [u8; 6] = [0x43, 0x65, 0x64, 0x61, 0x6c, 0x6f];

        let packet = Packet::make_publish(
            false,
            QosLevel::AtLeast,
            true,
            "info".into(),
            Some(2),
            Bytes::copy_from_slice(&payload),
        );

        let data: [u8; 16] = [
            0x33, // Fixed Header QOS 1, Retain 1
            0x0E, // Length 14
            0x00, 0x04, 0x69, 0x6e, 0x66, 0x6f, // topic "info"
            0x00, 0x02, // packet_id: 2
            0x43, 0x65, 0x64, 0x61, 0x6c, 0x6f, // Message "Cedalo"
        ];

        assert_eq!(data.to_vec(), packet);
    }

    #[test]
    fn test_pack_subscribe_packet() {
        let header = FixedHeader::new(
            super::enums::PacketType::Subscribe,
            false,
            QosLevel::AtLeast,
            false,
            12,
        );

        let v = VariableHeader::Subscribe {
            packet_id: 1,
            tuples: vec![("mytopic".into(), QosLevel::AtLeast)],
        };

        let packet = Packet::new(header, v).pack();

        let data: [u8; 14] = [
            0x82, // Header
            0x0C, // Len
            0x00, 0x01, // pkt id
            // Start Tubles
            0x00, 0x07, 0x6d, 0x79, 0x74, 0x6f, 0x70, 0x69, 0x63, // String "mytopic"
            0x01, // Qos
        ];

        assert_eq!(data.to_vec(), packet);
    }

    #[test]
    fn test_pack_unsubscribe_packet() {
        let header = FixedHeader::new(
            super::enums::PacketType::Unsubscribe,
            false,
            QosLevel::AtLeast,
            false,
            8,
        );

        let v = VariableHeader::Unsubscribe {
            packet_id: 1,
            tuples: vec!["info".into()],
        };

        let packet = Packet::new(header, v).pack();

        let data: [u8; 10] = [
            0xA2, // Fixed Header
            0x08, // length
            0x00, 0x01, // pkt_id = 1
            0x00, 0x04, 0x69, 0x6e, 0x66, 0x6f, // string "info"
        ];

        assert_eq!(data.to_vec(), packet);
    }
}
