use bytes::{BufMut, Bytes, BytesMut};

use crate::packets::{
    traits::{FromBytes, ToBytes},
    utils::unpack_u16,
};

/// Packets Puback,Pubrec,Pubrel,Pubcomp, Unsuback,PingReq,PingResp, Disconnect
/// all share the same format
#[derive(Debug)]
pub struct AckHeader {
    pub packet_id: u16,
}

impl AckHeader {
    pub fn new(id: u16) -> AckHeader {
        Self { packet_id: id }
    }
}

impl FromBytes for AckHeader {
    type Output = AckHeader;

    fn from_bytes<'a, I>(
        iter: &mut I,
        _: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, crate::error::MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let id = unpack_u16(iter)?;

        Ok(AckHeader { packet_id: id })
    }
}

impl ToBytes for AckHeader {
    fn to_bytes(&self) -> Result<Bytes, crate::error::MqttError> {
        let mut bytes = BytesMut::with_capacity(2);

        bytes.put_u16(self.packet_id);

        Ok(bytes.freeze())
    }
}
