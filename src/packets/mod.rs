use std::{io::Bytes, net::TcpStream};

use crate::error::MqttError;

use self::{
    enums::{ConnectReturnCode, QosLevel, SubackReturnCode},
    headers::{
        ack::AckHeader,
        connack::{AcknowledgeFlags, ConnackHeader},
        connect::ConnectHeader,
        fixed_header::FixedHeader,
        publish::PublishHeader,
        suback::SubackHeader,
        subscribe::SubscribeHeader,
        unsubscribe::UnsubscribeHeader,
    },
    traits::{FromBytes, ToBytes},
};

pub mod enums;
mod headers;
mod traits;
mod utils;

const MAX_LEN_BYTES: usize = 4;

#[derive(Debug)]
pub enum Packet {
    Connect(FixedHeader, ConnectHeader),
    ConnAck(FixedHeader, ConnackHeader),
    Subscribe(FixedHeader, SubscribeHeader),
    Unsubscribe(FixedHeader, UnsubscribeHeader),
    SubAck(FixedHeader, SubackHeader),
    Publish(FixedHeader, PublishHeader),
    /// A PUBACK Packet is the response to a PUBLISH Packet with QoS level 1.
    PubAck(FixedHeader, AckHeader),
    /// A PUBREC Packet is the response to a PUBLISH Packet with QoS 2. It is the second packet of the QoS 2 protocol exchange.
    PubRec(FixedHeader, AckHeader),
    /// A PUBREL Packet is the response to a PUBREC Packet. It is the third packet of the QoS 2 protocol exchange.
    PubRel(FixedHeader, AckHeader),
    PubComp(FixedHeader, AckHeader),
    UnsubAck(FixedHeader, AckHeader),
    PingReq(FixedHeader),
    PingResp(FixedHeader),
    Disconnect(FixedHeader),
}

impl Packet {
    pub fn make_publish(
        dup: bool,
        qos: QosLevel,
        retain: bool,
        topic: String,
        pkt_id: Option<u16>,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, MqttError> {
        let header = FixedHeader::new(enums::PacketType::Publish, dup, qos, retain, 0);

        let b = PublishHeader::new(topic, pkt_id, payload);

        Packet::Publish(header, b).pack()
    }

    pub fn make_pubcomp(packet_id: u16) -> Result<Vec<u8>, MqttError> {
        let header = FixedHeader::new(
            enums::PacketType::Puback,
            false,
            QosLevel::AtMostOnce,
            false,
            0,
        );

        Packet::PubComp(header, AckHeader::new(packet_id)).pack()
    }

    pub fn make_pubrel(packet_id: u16) -> Result<Vec<u8>, MqttError> {
        let header = FixedHeader::new(
            enums::PacketType::Puback,
            false,
            QosLevel::AtLeastOnce,
            false,
            0,
        );

        Packet::PubRel(header, AckHeader::new(packet_id)).pack()
    }

    pub fn make_pubrec(packet_id: u16) -> Result<Vec<u8>, MqttError> {
        let header = FixedHeader::new(
            enums::PacketType::Puback,
            false,
            QosLevel::AtMostOnce,
            false,
            0,
        );

        Packet::PubRec(header, AckHeader::new(packet_id)).pack()
    }

    pub fn make_puback(pkt_id: u16) -> Result<Vec<u8>, MqttError> {
        let header = FixedHeader::new(
            enums::PacketType::Puback,
            false,
            QosLevel::AtMostOnce,
            false,
            0,
        );

        Packet::PubAck(header, AckHeader::new(pkt_id)).pack()
    }

    pub fn make_unsuback(pkt_id: u16) -> Result<Vec<u8>, MqttError> {
        let header = FixedHeader::new(
            enums::PacketType::Unsuback,
            false,
            QosLevel::AtMostOnce,
            false,
            0,
        );

        Packet::UnsubAck(header, AckHeader::new(pkt_id)).pack()
    }

    pub fn make_ping_resp() -> Result<Vec<u8>, MqttError> {
        let header = FixedHeader::new(
            enums::PacketType::PingResp,
            false,
            enums::QosLevel::AtMostOnce,
            false,
            0,
        );

        Packet::PingResp(header).pack()
    }

    pub fn make_suback(
        packet_id: u16,
        rc_list: Vec<SubackReturnCode>,
    ) -> Result<Vec<u8>, MqttError> {
        let header = FixedHeader::new(
            enums::PacketType::Suback,
            false,
            enums::QosLevel::AtMostOnce,
            false,
            0,
        );

        Packet::SubAck(header, SubackHeader::new(packet_id, rc_list)).pack()
    }

    pub fn make_connack(
        rc: ConnectReturnCode,
        session_present: bool,
    ) -> Result<Vec<u8>, MqttError> {
        let flags = AcknowledgeFlags::new(session_present);
        let header = FixedHeader::new(
            enums::PacketType::Connack,
            false,
            enums::QosLevel::AtMostOnce,
            false,
            0,
        );
        let cack_header = ConnackHeader::new(flags, rc);
        Packet::ConnAck(header, cack_header).pack()
    }

    pub fn unpack(bytes: &[u8]) -> Result<Packet, MqttError> {
        let mut iter = bytes.iter();

        let header = FixedHeader::from_bytes(&mut iter, None)?;

        let packet_type = header.get_packet_type()?;

        match packet_type {
            enums::PacketType::Connect => Ok(Self::Connect(
                header,
                ConnectHeader::from_bytes(&mut iter, None)?,
            )),
            enums::PacketType::Connack => {
                let connack = ConnackHeader::from_bytes(&mut iter, None)?;
                Ok(Self::ConnAck(header, connack))
            }
            enums::PacketType::Publish => {
                let publish = PublishHeader::from_bytes(&mut iter, Some(&header))?;

                Ok(Self::Publish(header, publish))
            }
            enums::PacketType::Puback => Ok(Self::PubAck(
                header,
                AckHeader::from_bytes(&mut iter, None)?,
            )),
            enums::PacketType::Pubrec => Ok(Self::PubRec(
                header,
                AckHeader::from_bytes(&mut iter, None)?,
            )),
            enums::PacketType::Pubrel => Ok(Self::PubRel(
                header,
                AckHeader::from_bytes(&mut iter, None)?,
            )),
            enums::PacketType::Pubcomp => Ok(Self::PubComp(
                header,
                AckHeader::from_bytes(&mut iter, None)?,
            )),
            enums::PacketType::Subscribe => {
                let subscribe = SubscribeHeader::from_bytes(&mut iter, Some(&header))?;
                Ok(Self::Subscribe(header, subscribe))
            }
            enums::PacketType::Suback => {
                let h = SubackHeader::from_bytes(&mut iter, Some(&header))?;
                Ok(Self::SubAck(header, h))
            }
            enums::PacketType::Unsubscribe => {
                let unsubscribe = UnsubscribeHeader::from_bytes(&mut iter, Some(&header))?;
                Ok(Self::Unsubscribe(header, unsubscribe))
            }
            enums::PacketType::Unsuback => Ok(Self::UnsubAck(
                header,
                AckHeader::from_bytes(&mut iter, None)?,
            )),
            enums::PacketType::PingReq => Ok(Self::PingReq(header)),
            enums::PacketType::PingResp => Ok(Self::PingResp(header)),
            enums::PacketType::Disconnect => Ok(Self::Disconnect(header)),
            _ => Err(MqttError::InvalidPacketType),
        }
    }
    pub fn pack(&self) -> Result<Vec<u8>, MqttError> {
        match self {
            Packet::Connect(header, body) => {
                let vhp = body.to_bytes()?;
                let len = Packet::encode_length(vhp.len());

                let mut packet = [len, vhp].concat();
                packet.insert(0, header.as_byte());
                Ok(packet)
            }
            Packet::ConnAck(header, body) => {
                let vhp = body.to_bytes()?;
                let len = Packet::encode_length(vhp.len());

                let mut packet = [len, vhp].concat();
                packet.insert(0, header.as_byte());
                Ok(packet)
            }
            Packet::Subscribe(header, body) => {
                let vhp = body.to_bytes()?;
                let len = Packet::encode_length(vhp.len());

                let mut packet = [len, vhp].concat();
                packet.insert(0, header.as_byte());
                Ok(packet)
            }
            Packet::Unsubscribe(header, body) => {
                let vhp = body.to_bytes()?;
                let len = Packet::encode_length(vhp.len());

                let mut packet = [len, vhp].concat();
                packet.insert(0, header.as_byte());
                Ok(packet)
            }
            Packet::SubAck(header, body) => {
                let vhp = body.to_bytes()?;
                let len = Packet::encode_length(vhp.len());

                let mut packet = [len, vhp].concat();
                packet.insert(0, header.as_byte());
                Ok(packet)
            }
            Packet::Publish(header, body) => {
                let vhp = body.to_bytes()?;
                let len = Packet::encode_length(vhp.len());

                let mut packet = [len, vhp].concat();
                packet.insert(0, header.as_byte());
                Ok(packet)
            }

            Packet::UnsubAck(header, body)
            | Packet::PubComp(header, body)
            | Packet::PubRel(header, body)
            | Packet::PubAck(header, body)
            | Packet::PubRec(header, body) => {
                let vh_p = body.to_bytes()?;

                let len = Packet::encode_length(vh_p.len());

                let mut packet = [len, vh_p].concat();
                packet.insert(0, header.as_byte());

                Ok(packet)
            }

            Packet::Disconnect(header) | Packet::PingResp(header) | Packet::PingReq(header) => {
                Ok(vec![header.as_byte(), 0x00])
            }
        }
    }
    fn encode_length(len: usize) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::with_capacity(4);
        let mut mlen = len;
        loop {
            if bytes.len() + 1 > MAX_LEN_BYTES {
                return bytes;
            }

            let mut d = mlen % 128;
            mlen /= 128;

            if mlen > 0 {
                d |= 128;
            }

            bytes.push(d as u8);

            if !(mlen > 0) {
                break;
            }
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use crate::packets::{
        enums::QosLevel,
        headers::{
            publish::PublishHeader,
            subscribe::{SubscribeHeader, Tuple},
            unsubscribe::UnsubscribeHeader,
        },
    };

    use super::{
        headers::{connect::ConnectHeader, fixed_header::FixedHeader},
        Packet,
    };
    // https://cedalo.com/blog/mqtt-packet-guide/
    #[test]
    fn test_unpack_connect_packet() {
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

        let packet = Packet::unpack(&data).expect("Failed to parse connect packet");

        if let Packet::Connect(header, body) = packet {
            assert_eq!(header.get_remaing_len(), 30);

            assert_eq!(body.keepalive, 60);

            assert!(body.flags.clean_session());
            assert!(body.flags.has_password());
            assert!(body.flags.has_username());
            assert!(!body.flags.will());
            assert_eq!(body.flags.will_qos().expect("QOS"), QosLevel::AtMostOnce);
            assert!(!body.flags.will_retain());

            assert_eq!(body.username.len(), 6, "Body Username is not of length '4'");
            assert_eq!(body.password.len(), 4, "Body Password is not of length '4'");
            assert_eq!(&body.client_id, "myPy");
            assert_eq!(&body.password, "pass");
            assert_eq!(&body.username, "client");

            assert_eq!(
                body.flags.will_qos().expect("Failed to get qos"),
                QosLevel::AtMostOnce
            );

            assert_eq!(body.keepalive, 60)
        } else {
            panic!("Packet was not a connect packet");
        }
    }

    #[test]
    fn test_unpack_publish_packet() {
        let data: [u8; 16] = [
            0x33, // Fixed Header QOS 1, Retain 1
            0x0E, // Length 14
            0x00, 0x04, 0x69, 0x6e, 0x66, 0x6f, // topic "info"
            0x00, 0x02, // packet_id: 2
            0x43, 0x65, 0x64, 0x61, 0x6c, 0x6f, // Message "Cedalo"
        ];

        let packet = Packet::unpack(&data).expect("Failed to parse connect packet");

        if let Packet::Publish(header, body) = packet {
            assert_eq!(header.get_remaing_len(), 14);

            assert_eq!(
                header.get_qos().expect("Failed to get QOS"),
                QosLevel::AtLeastOnce
            );
            assert!(header.get_retain());

            assert_eq!(&body.topic, "info");

            assert!(body.packet_id.is_some());
            assert_eq!(body.packet_id.expect("Failed to get packet id"), 2);

            assert_eq!(body.payload.len(), 6);

            let s = String::from_utf8(body.payload).expect("Failed to parse string");

            assert_eq!(&s, "Cedalo");
        } else {
            panic!("Invalid packet type");
        }
    }

    #[test]
    fn test_unpack_subscribe_packet() {
        let data: [u8; 14] = [
            0x82, // Header
            0x0C, // Len
            0x00, 0x01, // pkt id
            // Start Tubles
            0x00, 0x07, 0x6d, 0x79, 0x74, 0x6f, 0x70, 0x69, 0x63, // String "mytopic"
            0x01, // Qos
        ];

        let packet = Packet::unpack(&data).expect("Failed to parse connect packet");

        if let Packet::Subscribe(header, body) = packet {
            assert_eq!(header.get_remaing_len(), 12);

            assert_eq!(body.packet_id, 1);

            assert_eq!(body.tuples.len(), 1);
            assert_eq!(body.tuples[0].topic, "mytopic");
            assert_eq!(body.tuples[0].qos, QosLevel::AtLeastOnce);
        } else {
            panic!("Invalid packet");
        }
    }

    #[test]
    fn test_unpack_unsubscribe_packet() {
        let data: [u8; 10] = [
            0xA2, // Fixed Header
            0x08, // length
            0x00, 0x01, // pkt_id = 1
            0x00, 0x04, 0x69, 0x6e, 0x66, 0x6f, // string "info"
        ];

        let packet = Packet::unpack(&data).expect("Failed to parse connect packet");

        if let Packet::Unsubscribe(header, body) = packet {
            assert_eq!(header.get_remaing_len(), 8, "remaing packet length");

            assert_eq!(body.packet_id, 1);

            assert_eq!(body.tuples.len(), 1);
            assert_eq!(body.tuples[0], "info");
        } else {
            panic!("Invalid packet");
        }
    }

    #[test]
    fn test_pack_connect_packet() {
        let header = FixedHeader::new(
            super::enums::PacketType::Connect,
            false,
            QosLevel::AtMostOnce,
            false,
            30,
        );

        let flags = crate::packets::headers::connect::Flags::new(
            true,
            false,
            QosLevel::AtMostOnce,
            false,
            true,
            true,
        );
        let h = ConnectHeader::new(
            flags,
            60,
            "myPy".into(),
            Some("client".into()),
            Some("pass".into()),
            None,
            None,
        );

        let packet = Packet::Connect(header, h);

        let bytes = packet.pack().expect("Packet failed to pack");

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
        let header = FixedHeader::new(
            super::enums::PacketType::Publish,
            false,
            QosLevel::AtLeastOnce,
            true,
            14,
        );

        let p = PublishHeader::new(
            "info".into(),
            Some(2),
            vec![0x43, 0x65, 0x64, 0x61, 0x6c, 0x6f],
        );

        let packet = Packet::Publish(header, p)
            .pack()
            .expect("Failed to pack packet");

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
            QosLevel::AtLeastOnce,
            false,
            12,
        );

        let a = SubscribeHeader::new(1, vec![Tuple::new("mytopic".into(), QosLevel::AtLeastOnce)]);

        let packet = Packet::Subscribe(header, a)
            .pack()
            .expect("Failed to convert packet");

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
            QosLevel::AtLeastOnce,
            false,
            8,
        );

        let uh = UnsubscribeHeader::new(1, vec!["info".into()]);

        let packet = Packet::Unsubscribe(header, uh)
            .pack()
            .expect("Failed to pack");

        let data: [u8; 10] = [
            0xA2, // Fixed Header
            0x08, // length
            0x00, 0x01, // pkt_id = 1
            0x00, 0x04, 0x69, 0x6e, 0x66, 0x6f, // string "info"
        ];

        assert_eq!(data.to_vec(), packet);
    }

    #[test]
    fn test_encode_single_byte() {
        let len: usize = 0x1e;

        let result = Packet::encode_length(len);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 30);
    }

    #[test]
    fn test_encode_two_bytes() {
        let len: usize = 321;

        let result = Packet::encode_length(len);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 193);
        assert_eq!(result[1], 2);
    }
}
