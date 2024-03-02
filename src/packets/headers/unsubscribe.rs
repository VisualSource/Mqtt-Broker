use std::mem::size_of;

use crate::{
    error::MqttError,
    packets::{
        traits::{FromBytes, ToBytes},
        utils::{self, unpack_string, unpack_u16},
    },
};

#[derive(Debug, Default)]
pub struct UnsubscribeHeader {
    pub packet_id: u16,
    pub tuples: Vec<String>,
}

impl UnsubscribeHeader {
    pub fn new(packet_id: u16, tuples: Vec<String>) -> Self {
        Self { packet_id, tuples }
    }
}

impl FromBytes for UnsubscribeHeader {
    type Output = UnsubscribeHeader;

    fn from_byte_stream(
        iter: &mut std::io::Bytes<&mut std::net::TcpStream>,
        header: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, MqttError> {
        let h = header.ok_or_else(|| MqttError::MissingFixedHeader)?;
        let mut len = h.get_remaing_len();
        let mut unsub = UnsubscribeHeader::default();

        unsub.packet_id = utils::stream::unpack_u16(iter)?;
        len -= size_of::<u16>();

        while len > 0 {
            len -= size_of::<u16>();

            let topic = utils::stream::unpack_string(iter)?;
            len -= topic.len();

            unsub.tuples.push(topic);
        }

        Ok(unsub)
    }

    fn from_bytes<'a, I>(
        iter: &mut I,
        header: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, crate::error::MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let h = header.ok_or_else(|| MqttError::MissingFixedHeader)?;

        let mut len = h.get_remaing_len();
        let mut unsub = UnsubscribeHeader::default();

        unsub.packet_id = unpack_u16(iter)?;
        len -= size_of::<u16>();

        while len > 0 {
            len -= size_of::<u16>();

            let topic = unpack_string(iter)?;
            len -= topic.len();

            unsub.tuples.push(topic);
        }

        Ok(unsub)
    }
}

impl ToBytes for UnsubscribeHeader {
    fn to_bytes(&self) -> Result<Vec<u8>, MqttError> {
        let mut data = vec![self.packet_id.to_be_bytes().to_vec()];

        for x in &self.tuples {
            data.push((x.len() as u16).to_be_bytes().to_vec());
            data.push(x.as_bytes().to_vec());
        }

        Ok(data.concat())
    }
}
