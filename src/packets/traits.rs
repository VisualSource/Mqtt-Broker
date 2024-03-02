use std::{io::Bytes, net::TcpStream};

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

    fn from_byte_stream(
        iter: &mut Bytes<&mut TcpStream>,
        header: Option<&FixedHeader>,
    ) -> Result<Self::Output, MqttError>;
}

pub trait ToBytes {
    fn to_bytes(&self) -> Result<Vec<u8>, MqttError>;
}
