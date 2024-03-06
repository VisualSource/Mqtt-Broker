use std::mem::size_of;

use bytes::{BufMut, Bytes, BytesMut};

use crate::{
    error::MqttError,
    packets::{
        enums::QosLevel,
        traits::{FromBytes, ToBytes},
        utils::{unpack_string, unpack_u16},
    },
};

#[derive(Debug, Default)]
pub struct SubscribeHeader {
    pub packet_id: u16,
    pub tuples: Vec<(String, QosLevel)>,
}

impl SubscribeHeader {
    pub fn new(packet_id: u16, tuples: Vec<(String, QosLevel)>) -> Self {
        Self { packet_id, tuples }
    }
    pub fn builder() -> Self {
        Self {
            packet_id: 0,
            tuples: Vec::new(),
        }
    }
}

impl FromBytes for SubscribeHeader {
    type Output = SubscribeHeader;

    fn from_bytes<'a, I>(
        iter: &mut I,
        header: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, crate::error::MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let h = header.ok_or_else(|| MqttError::MissingFixedHeader)?;

        let mut len = h.get_remaing_len();

        let mut sub = SubscribeHeader::builder();

        sub.packet_id = unpack_u16(iter)?;
        len -= size_of::<u16>();

        /*
         * Read in a loop all remaining bytes specified by len of the Fixed Header.
         * From now on the payload consists of 3-tuples formed by:
         *  - topic filter (string)
         *  - qos
         */

        while len > 0 {
            len -= size_of::<u16>();

            let topic = unpack_string(iter)?;
            len -= topic.len();

            let qos = QosLevel::try_from(*iter.next().ok_or_else(|| MqttError::MissingByte)?)?;

            len -= size_of::<u8>();

            sub.tuples.push((topic, qos));
        }

        Ok(sub)
    }
}

impl ToBytes for SubscribeHeader {
    fn to_bytes(&self) -> Result<Bytes, MqttError> {
        let mut bytes = BytesMut::new();

        bytes.put_u16(self.packet_id);

        for (topic, qos) in &self.tuples {
            bytes.put_u16(topic.len() as u16);
            bytes.put(topic.as_bytes());
            bytes.put_u8(*qos as u8);
        }

        Ok(bytes.freeze())
    }
}
