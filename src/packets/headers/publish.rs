use std::mem::size_of;

use bytes::Bytes;

use crate::{
    error::MqttError,
    packets::{
        enums::QosLevel,
        traits::{FromBytes, ToBytes},
        utils::{unpack_bytes, unpack_string, unpack_u16},
    },
};

#[derive(Debug, Default)]
pub struct PublishHeader {
    pub topic: String,
    pub packet_id: Option<u16>,
    pub payload: Bytes,
}

impl PublishHeader {
    pub fn builder() -> Self {
        Self {
            topic: String::default(),
            packet_id: None,
            payload: Bytes::new(),
        }
    }
    pub fn new(topic: String, packet_id: Option<u16>, payload: Bytes) -> Self {
        Self {
            topic,
            packet_id,
            payload,
        }
    }
}

impl FromBytes for PublishHeader {
    type Output = PublishHeader;

    fn from_bytes<'a, I>(
        iter: &mut I,
        header: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, crate::error::MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let mut publish = PublishHeader::builder();

        publish.topic = unpack_string(iter)?;

        let h = header.ok_or_else(|| MqttError::MissingFixedHeader)?;

        /*
         * Message len is calculated subtracting the length of the variable header
         * from the Remaining Length field that is in the Fixed Header
         */
        let mut len = h.get_remaing_len();

        if h.get_qos()? > QosLevel::AtMostOnce {
            publish.packet_id = Some(unpack_u16(iter)?);
            len -= size_of::<u16>();
        }

        len -= size_of::<u16>() + publish.topic.len();

        publish.payload = unpack_bytes(iter, len)?;

        Ok(publish)
    }
}

impl ToBytes for PublishHeader {
    fn to_bytes(&self) -> Result<Vec<u8>, MqttError> {
        let mut data = vec![
            (self.topic.len() as u16).to_be_bytes().to_vec(),
            self.topic.as_bytes().to_vec(),
        ];

        if let Some(id) = self.packet_id {
            data.push(id.to_be_bytes().to_vec());
        }

        data.push(self.payload.to_vec());

        Ok(data.concat())
    }
}
