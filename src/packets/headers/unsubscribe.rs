use std::mem::size_of;

use bytes::{BufMut, Bytes, BytesMut};

use crate::{
    error::MqttError,
    packets::{
        traits::{FromBytes, ToBytes},
        utils::{unpack_string, unpack_u16},
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
    fn to_bytes(&self) -> Result<Bytes, MqttError> {
        let mut bytes = BytesMut::new();

        bytes.put_u16(self.packet_id);

        for x in &self.tuples {
            bytes.put_u16(x.len() as u16);
            bytes.put(x.as_bytes());
        }

        Ok(bytes.freeze())
    }
}
