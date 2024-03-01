use crate::utils::{unpack_u16, FromBytes};

use super::{enums::QosLevel, fixed_header::MqttFixedHeader};

pub struct MqttPublish {
    header: MqttFixedHeader,
    packet_id: u16,
    topiclen: u16,
    topic: String,
    payload_len: u16,
    payload: String,
}

impl FromBytes for MqttPublish {
    type Output = MqttPublish;

    fn from_bytes<'a, I>(mut iter: I, header: MqttFixedHeader) -> Result<Self::Output, ()>
    where
        I: Iterator<Item = &'a u8>,
    {
        let topic_len = unpack_u16(&mut iter)?;
        let mut msg_len = topic_len;

        let qos = header.get_qos()?;

        if qos > QosLevel::AtMostOnce {
            let pack_id = unpack_u16(&mut iter)?;
        }

        return Ok(MqttPublish {
            header: header,
            packet_id: 0,
            topiclen: 0,
            topic: String::new(),
            payload_len: 0,
            payload: String::new(),
        });
    }
}
