use std::{io::Read, usize};

use crate::{
    consts::MAX_LEN_BYTES,
    headers::{
        fixed_header::MqttFixedHeader, mqtt_ack, mqtt_connack::MqttConnack,
        mqtt_connect::MqttConnect, mqtt_publish::MqttPublish, mqtt_sub::MqttSubHeader,
        mqtt_suback::MqttSuback, MqttPingreg, MqttPingresp, MqttPuback, MqttPubcomp, MqttPubrec,
        MqttPubrel, MqttSubscribe, MqttUnsuback, MqttUnsubscribe,
    },
    utils::FromBytes,
};

fn encode_len(len: u8) -> [u8; MAX_LEN_BYTES] {
    let mut bytes = 0;
    let mut buf: [u8; MAX_LEN_BYTES] = [0, 0, 0, 0];
    let mut mlen = len;
    loop {
        if bytes + 1 > MAX_LEN_BYTES {
            return buf;
        }

        let mut d = mlen % 128;
        mlen /= 128;

        if mlen > 0 {
            d |= 128;
        }

        // buffer
        buf[bytes] = d;
        bytes += 1;
        if mlen > 0 {
            break;
        }
    }

    buf
}

fn decode_len<'a, I>(iter: &mut I) -> Result<u32, ()>
where
    I: Iterator<Item = &'a u8>,
{
    let mut multiplier: u32 = 1;
    let mut value: u32 = 0;

    // do while loop
    loop {
        let byte = iter.next().ok_or_else(|| ())?;
        value += ((byte.to_owned() as u32) & 127) * multiplier;
        if multiplier > (128 * 128 * 128) {
            return Err(());
        }
        multiplier *= 128;
        if (byte & 128) != 0 {
            return Ok(value);
        }
    }
}

pub enum Packet {
    Connect(MqttConnect),
    Connack(MqttConnack),
    Suback(MqttSuback),
    Publish(MqttPublish),
    Subscribe(MqttSubscribe),
    Unsubscribe(MqttUnsubscribe),
    Pubrel(MqttPubrel),
    Puback(MqttPuback),
    Pubrec(MqttPubrec),
    Pubcomp(MqttPubcomp),
    PubUnsuback(MqttUnsuback),
    PingReq(MqttPingreg),
    PingResp(MqttPingresp),
    Disconnect(MqttFixedHeader),
}

impl TryFrom<&[u8]> for Packet {
    type Error = ();

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut byte_iter = value.iter();

        let header_byte = byte_iter.next().ok_or_else(|| ())?;

        let header = MqttFixedHeader::from(header_byte);

        // decode remaing length field
        let len = decode_len(&mut byte_iter)?;

        match header.get_type()? {
            crate::headers::enums::PacketType::CONNECT => {
                Ok(Packet::Connect(MqttConnect::from_bytes(byte_iter, header)?))
            }
            crate::headers::enums::PacketType::CONNACK => {
                Ok(Packet::Connack(MqttConnack::from_bytes(byte_iter, header)?))
            }
            crate::headers::enums::PacketType::PUBLISH => {
                Ok(Packet::Publish(MqttPublish::from_bytes(byte_iter, header)?))
            }
            crate::headers::enums::PacketType::PUBACK => {
                Ok(Packet::Puback(MqttPuback::from_bytes(byte_iter, header)?))
            }
            crate::headers::enums::PacketType::PUBREC => {
                Ok(Packet::Pubrec(MqttPubrec::from_bytes(byte_iter, header)?))
            }
            crate::headers::enums::PacketType::PUBREL => {
                Ok(Packet::Pubrel(MqttPubrel::from_bytes(byte_iter, header)?))
            }
            crate::headers::enums::PacketType::PUBCOMP => {
                Ok(Packet::Pubcomp(MqttPubcomp::from_bytes(byte_iter, header)?))
            }
            crate::headers::enums::PacketType::SUBSCRIBE => Ok(Packet::Subscribe(
                MqttSubscribe::from_bytes(byte_iter, header)?,
            )),
            crate::headers::enums::PacketType::SUBACK => {
                Ok(Packet::Suback(MqttSuback::from_bytes(byte_iter, header)?))
            }
            crate::headers::enums::PacketType::UNSUBSCRIBE => Ok(Packet::Unsubscribe(
                MqttUnsubscribe::from_bytes(byte_iter, header)?,
            )),
            crate::headers::enums::PacketType::UNSUBACK => Ok(Packet::PubUnsuback(
                MqttUnsuback::from_bytes(byte_iter, header)?,
            )),
            crate::headers::enums::PacketType::PINGREQ => Ok(Packet::PingReq(header)),
            crate::headers::enums::PacketType::PINGRESP => Ok(Packet::PingResp(header)),
            crate::headers::enums::PacketType::DISCONNECT => Ok(Packet::Disconnect(header)),
        }
    }
}
