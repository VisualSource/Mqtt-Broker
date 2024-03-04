use std::mem::size_of;

use crate::{
    error::MqttError,
    packets::{
        enums::SubackReturnCode,
        traits::{FromBytes, ToBytes},
        utils::unpack_u16,
    },
};

#[derive(Debug, Default)]
pub struct SubackHeader {
    pub packet_id: u16,
    pub return_codes: Vec<SubackReturnCode>,
}

impl SubackHeader {
    pub fn new(packet_id: u16, return_codes: Vec<SubackReturnCode>) -> Self {
        Self {
            packet_id,
            return_codes,
        }
    }
}

impl FromBytes for SubackHeader {
    type Output = SubackHeader;
    fn from_bytes<'a, I>(
        iter: &mut I,
        header: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, crate::error::MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let h = header.ok_or_else(|| MqttError::MissingFixedHeader)?;
        let mut item = SubackHeader::default();

        item.packet_id = unpack_u16(iter)?;

        let mut len = h.get_remaing_len() - size_of::<u16>();

        while len > 0 {
            let byte = iter.next().ok_or_else(|| MqttError::MissingByte)?;

            item.return_codes.push(SubackReturnCode::try_from(byte)?);

            len -= size_of::<u8>()
        }

        Ok(item)
    }
}

impl ToBytes for SubackHeader {
    fn to_bytes(&self) -> Result<Vec<u8>, MqttError> {
        let data = vec![
            self.packet_id.to_be_bytes().to_vec(),
            self.return_codes.iter().map(|x| x.to_u8()).collect(),
        ];

        Ok(data.concat())
    }
}
