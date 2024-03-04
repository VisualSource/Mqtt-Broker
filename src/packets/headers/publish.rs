use std::mem::size_of;

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
    pub payload: Vec<u8>,
}

impl PublishHeader {
    pub fn new(topic: String, packet_id: Option<u16>, payload: Vec<u8>) -> Self {
        Self {
            topic,
            packet_id: packet_id,
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
        let mut publish = PublishHeader::default();

        publish.topic = unpack_string(iter)?;

        let h = header.ok_or_else(|| MqttError::MissingFixedHeader)?;

        /*
         * Message len is calculated subtracting the length of the variable header
         * from the Remaining Length field that is in the Fixed Header
         */
        let mut len = h.get_remaing_len() as usize;

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

        data.push(self.payload.to_owned());

        Ok(data.concat())
    }
}
