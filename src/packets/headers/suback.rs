use std::mem::size_of;

use crate::{
    error::MqttError,
    packets::{
        traits::{FromBytes, ToBytes},
        utils::{self, unpack_u16},
    },
};

#[repr(u8)]
#[derive(Debug)]
pub enum ReturnCode {
    SuccessQosZero = 0x00,
    SuccessQosOne = 0x01,
    SuccessQosTwo = 0x02,
    Failure = 0x80,
}
impl ReturnCode {
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::SuccessQosZero => 0x00,
            Self::SuccessQosOne => 0x01,
            Self::SuccessQosTwo => 0x02,
            Self::Failure => 0x80,
        }
    }
}

impl TryFrom<u8> for ReturnCode {
    type Error = MqttError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        ReturnCode::try_from(&value)
    }
}

impl TryFrom<&u8> for ReturnCode {
    type Error = MqttError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::SuccessQosZero),
            0x01 => Ok(Self::SuccessQosOne),
            0x02 => Ok(Self::SuccessQosTwo),
            0x80 => Ok(Self::Failure),
            _ => Err(MqttError::Convertion(
                value.to_string(),
                "ReturnCode".into(),
            )),
        }
    }
}

#[derive(Debug, Default)]
pub struct SubackHeader {
    pub packet_id: u16,
    pub return_codes: Vec<ReturnCode>,
}

impl FromBytes for SubackHeader {
    type Output = SubackHeader;

    fn from_byte_stream(
        iter: &mut std::io::Bytes<&mut std::net::TcpStream>,
        header: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, MqttError> {
        let h = header.ok_or_else(|| MqttError::MissingFixedHeader)?;
        let mut item = SubackHeader::default();
        item.packet_id = utils::stream::unpack_u16(iter)?;
        let mut len = h.get_remaing_len() - size_of::<u16>();

        while len > 0 {
            let byte = iter
                .next()
                .ok_or_else(|| MqttError::MalformedHeader)?
                .map_err(|e| MqttError::ByteRead(e))?;

            item.return_codes.push(ReturnCode::try_from(byte)?);

            len -= size_of::<u8>()
        }

        Ok(item)
    }

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

            item.return_codes.push(ReturnCode::try_from(byte)?);

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
