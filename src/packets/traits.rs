use bytes::Bytes;

use crate::error::MqttError;

use super::headers::fixed_header::FixedHeader;

pub trait FromBytes {
    type Output;
    fn from_bytes<'a, I>(
        iter: &mut I,
        header: Option<&FixedHeader>,
    ) -> Result<Self::Output, MqttError>
    where
        I: Iterator<Item = &'a u8>;
}

pub trait ToBytes {
    fn to_bytes(&self) -> Result<Bytes, MqttError>;
}
