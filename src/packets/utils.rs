use bytes::{BufMut, Bytes, BytesMut};

use crate::error::MqttError;
use std::mem::size_of;
/// Unpack a bytes into a u16
///
/// Integer data values are 16 bits in big-endian order: the high order byte precedes the lower order byte.
/// This means that a 16-bit word is presented on the network as Most Significant Byte (MSB), followed by Least Significant Byte (LSB).
pub fn unpack_u16<'a, I>(iter: &mut I) -> Result<u16, MqttError>
where
    I: Iterator<Item = &'a u8>,
{
    let bytes: [u8; size_of::<u16>()] = iter
        .take(size_of::<u16>())
        .map(|e| *e)
        .collect::<Vec<u8>>()
        .try_into()
        .map_err(|_| MqttError::MissingByte)?;

    Ok(u16::from_be_bytes(bytes))
}

/// Unpack bytes into a u32
///
/// Four Byte Integer data values are 32-bit unsigned integers in big-endian order: the high order byte precedes the successively lower order bytes. This means that a 32-bit word is presented on the network as Most Significant Byte (MSB), followed by the next most Significant Byte (MSB), followed by the next most Significant Byte (MSB), followed by Least Significant Byte (LSB).
pub fn unpack_u32<'a, I>(iter: &mut I) -> Result<u32, MqttError>
where
    I: Iterator<Item = &'a u8>,
{
    let bytes: [u8; size_of::<u32>()] = iter
        .take(size_of::<u32>())
        .copied()
        .collect::<Vec<u8>>()
        .try_into()
        .map_err(|_| MqttError::MissingByte)?;

    Ok(u32::from_be_bytes(bytes))
}

/// Unpack a UTF-8 string with u16 header.
/// Unless stated otherwise all UTF-8 encoded strings can have any length in the range 0 to 65535 bytes.
///
/// | Bit    | 7  6  5  4  3  2  1 |
/// | ------ | :-  -  -  -  -  -  -:     |
/// | byte 1 | String length MSB         |
/// | byte 2 | String length LSB         |
/// | byte 3 | UTF-8 encoded Character data |
///
/// [(MQTT 3.1.1) UTF-8 encoded strings](http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/errata01/os/mqtt-v3.1.1-errata01-os-complete.html#_Toc442180829)
pub fn unpack_string<'a, I>(iter: &mut I) -> Result<String, MqttError>
where
    I: Iterator<Item = &'a u8>,
{
    let len = usize::from(unpack_u16(iter)?);

    unpack_string_with_len(iter, len)
}

/// Read a set of bytes
pub fn unpack_bytes<'a, I>(iter: &mut I, len: usize) -> Result<bytes::Bytes, MqttError>
where
    I: Iterator<Item = &'a u8>,
{
    Ok(Bytes::from_iter(iter.take(len).copied()))
}

pub fn unpack_string_with_len<'a, I>(iter: &mut I, len: usize) -> Result<String, MqttError>
where
    I: Iterator<Item = &'a u8>,
{
    if len == 0 {
        return Ok(String::default());
    }

    let chars: Vec<_> = iter.take(len).map(|e| e.to_owned()).collect();

    if chars.len() != len || len > 65535 {
        return Err(MqttError::InvalidLength);
    }

    let result = String::from_utf8(chars).map_err(MqttError::Utf8Error)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::packets::utils::unpack_u32;

    use super::{unpack_string, unpack_u16};

    #[test]
    fn test_unpack_u32() {
        let input: [u8; 4] = [0x00, 0x01, 0xD9, 0xF6];

        let mut iter = input.iter();

        let result = unpack_u32(&mut iter).unwrap();

        assert_eq!(result, 121334)
    }

    #[test]
    fn test_unpack_u16() {
        let input: [u8; 2] = [0x00, 0xC];

        let mut iter = input.iter();

        let result = unpack_u16(&mut iter).unwrap();

        assert_eq!(result, 12)
    }

    #[test]
    fn test_unpack_a() {
        let input: [u8; 2] = [0x00, 0x0a];

        let mut iter = input.iter();

        let result = unpack_u16(&mut iter).expect("Failed to parse");

        assert_eq!(result, 10)
    }

    #[test]
    fn test_unpack_error() {
        let input: [u8; 1] = [0xC];

        let mut iter = input.iter();

        unpack_u16(&mut iter).expect_err("Testing Expect Error");
    }

    #[test]
    fn test_unpack_string() {
        //  'Hello, World' UTF8 string with u16 length header, in big endian order
        let input: [u8; 14] = [
            0x00, 0xC, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64,
        ];

        let mut iter = input.iter();

        let result = unpack_string(&mut iter).unwrap();

        assert_eq!(result, "Hello, World".to_string())
    }
}
