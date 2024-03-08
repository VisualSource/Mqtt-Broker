use bytes::{BufMut, Bytes, BytesMut};
use log::error;

use crate::error::MqttError;
use std::mem::size_of;
const MAX_ENCODED_SIZE: usize = 128 * 128 * 128;
const MAX_ENCODED_BYTES: usize = 4;

fn format_unpack_u16_error(e: Vec<u8>) -> MqttError {
    error!(
        "Failed to unpack u16: length is too short recived: {} expected {}",
        e.len(),
        size_of::<u16>()
    );
    MqttError::RequiredByteMissing
}

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
        .copied()
        .collect::<Vec<u8>>()
        .try_into()
        .map_err(format_unpack_u16_error)?;

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

/// ### UTF-8 Encoded String
/// Text fields within the MQTT Control Packets described later are encoded as UTF-8 strings.
/// UTF-8 [RFC3629](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#RFC3629) is an efficient encoding of Unicode [Unicode](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#Unicode) characters that optimizes the encoding of ASCII characters in support of text-based communications.
///
/// Each of these strings is prefixed with a Two Byte Integer length field that gives the number of bytes in a UTF-8 encoded string itself,
/// as illustrated in **Figure 1.1 Structure of UTF-8 Encoded Strings below**. Consequently, the maximum size of a UTF-8 Encoded String is 65,535 bytes.
///
/// Unless stated otherwise all UTF-8 encoded strings can have any length in the range 0 to 65,535 bytes.
///
/// Figure 1-1 Structure of UTF-8 Encoded Strings
///
/// | Bit    | 7  6  5  4  3  2  1 |
/// | ------ | :-  -  -  -  -  -  -:     |
/// | byte 1 | String length MSB         |
/// | byte 2 | String length LSB         |
/// | byte 3 | UTF-8 encoded Character data |
///
/// [(MQTT 3.1.1) 1.5.3 UTF-8 encoded strings](http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/errata01/os/mqtt-v3.1.1-errata01-os-complete.html#_Toc442180829)<br/>
/// [(MQTT 5) 1.5.4 UTF-8 Encoded String](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010)
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

    let chars: Vec<u8> = iter.take(len).copied().collect();

    let result = String::from_utf8(chars).map_err(MqttError::MalformedString)?;

    Ok(result)
}

pub fn decode_length<'a, I>(iter: &mut I) -> Result<(usize, usize), MqttError>
where
    I: Iterator<Item = &'a u8>,
{
    let mut multiplier: usize = 1;
    let mut value = 0;
    let mut len = 0;
    loop {
        let byte = iter.next().ok_or_else(|| MqttError::MissingByte)?;

        len += 1;

        value += ((byte & 127) as usize) * multiplier;

        if multiplier > MAX_ENCODED_SIZE {
            return Err(MqttError::MalformedRemaingLength);
        }

        multiplier *= 128;

        if (byte & 128) == 0 {
            break;
        }
    }

    Ok((value, len))
}

pub fn encode_length(len: usize, bytes: &mut BytesMut) {
    let mut mlen = len;
    loop {
        if bytes.len() + 1 > MAX_ENCODED_BYTES {
            return;
        }

        let mut d = mlen % 128;
        mlen /= 128;

        if mlen > 0 {
            d |= 128;
        }

        bytes.put_u8(d as u8);

        if mlen == 0 {
            break;
        }
    }
}

/// ### UTF-8 String Pair
/// A UTF-8 String Pair consists of two UTF-8 Encoded Strings. This data type is used to hold name-value pairs. The first string serves as the name, and the second string contains the value.
/// ***Both strings MUST comply with the requirements for UTF-8 Encoded Strings*** [MQTT-1.5.7-1]. If a receiver (Client or Server) receives a string pair which does not meet these requirements it is a Malformed Packet. Refer to section 4.13 for information about handling errors.
pub fn unpack_string_pair<'a, I>(iter: &mut I) -> Result<(String, String), MqttError>
where
    I: Iterator<Item = &'a u8>,
{
    let key = unpack_string(iter)?;
    let value = unpack_string(iter)?;

    Ok((key, value))
}

#[derive(Debug, Default)]
pub struct Props {
    payload_format_indicator: Option<bool>,
    message_expriy_interval: Option<u32>,
    content_type: Option<String>,
    response_topic: Option<String>,
    correlation_data: Option<Bytes>,
    session_expiry_interval: Option<u32>,
    assigned_client_identifier: Option<String>,
    server_keep_alive: Option<u16>,
    authentication_method: Option<String>,
    authenication_data: Option<Bytes>,
    request_problem_infomation: Option<bool>,
    will_delay_interval: Option<u32>,
    request_response_information: Option<bool>,
    response_infomation: Option<String>,
    server_reference: Option<String>,
    reason_string: Option<String>,
    reveive_maximum: Option<u16>,
    topic_alias_maximum: Option<u16>,
    topic_alias: Option<u16>,
    maximum_qos: Option<u8>,
    retain_available: Option<bool>,
    user_property: Option<Vec<(String, String)>>,
    maximum_packet_size: Option<u32>,
    wildcard_subscription_available: Option<bool>,
    subscription_identifier_available: Option<bool>,
    shared_subscription_available: Option<bool>,
    subscription_identifer: Option<usize>,
}

pub fn unpack_properties<'a, I>(iter: &mut I) -> Result<Props, MqttError>
where
    I: Iterator<Item = &'a u8>,
{
    let mut props = Props::default();
    let mut props_len = decode_length(iter)?.0;

    while props_len > 0 {
        let byte = iter.next().ok_or_else(|| MqttError::MissingByte)?;

        match byte {
            // Byte
            0x01 | 0x17 | 0x19 | 0x24 | 0x25 | 0x28 | 0x29 | 0x2A => {
                let data = iter.next().ok_or_else(|| MqttError::MissingByte)?;

                match byte {
                    0x01 => {
                        if props.payload_format_indicator.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.payload_format_indicator = Some(*data == 1);
                    }
                    0x17 => {
                        if *data > 1 || props.request_problem_infomation.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }

                        props.request_problem_infomation = Some(*data == 1);
                    }
                    0x19 => {
                        if *data > 1 || props.request_response_information.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.request_response_information = Some(*data == 1);
                    }
                    0x24 => {
                        if props.maximum_qos.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.maximum_qos = Some(*data);
                    }
                    0x25 => {
                        if props.retain_available.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.retain_available = Some(*data == 1);
                    }
                    0x28 => {
                        if props.wildcard_subscription_available.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.wildcard_subscription_available = Some(*data == 1);
                    }
                    0x29 => {
                        if props.subscription_identifier_available.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.subscription_identifier_available = Some(*data == 1);
                    }
                    0x2A => {
                        if props.shared_subscription_available.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.shared_subscription_available = Some(*data == 1);
                    }
                    _ => return Err(MqttError::MalformedHeader),
                }

                props_len -= 1;
            }
            // Two Byte Integer
            0x13 | 0x21 | 0x22 | 0x23 => {
                let data = unpack_u16(iter)?;
                props_len -= size_of::<u16>();

                match byte {
                    0x13 => {
                        if props.server_keep_alive.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.server_keep_alive = Some(data);
                    }
                    0x21 => {
                        if props.reveive_maximum.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.reveive_maximum = Some(data);
                    }
                    0x22 => {
                        if props.topic_alias_maximum.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.topic_alias_maximum = Some(data);
                    }
                    0x23 => {
                        if props.topic_alias.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.topic_alias = Some(data);
                    }
                    _ => return Err(MqttError::MalformedHeader),
                }
            }
            // Four Byte Integer
            0x02 | 0x11 | 0x18 | 0x27 => {
                let data = unpack_u32(iter)?;
                props_len -= size_of::<u32>();
                match byte {
                    0x02 => {
                        if props.message_expriy_interval.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.message_expriy_interval = Some(data);
                    }
                    0x11 => {
                        if props.session_expiry_interval.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.session_expiry_interval = Some(data);
                    }
                    0x18 => {
                        if props.will_delay_interval.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.will_delay_interval = Some(data);
                    }
                    0x27 => {
                        if props.maximum_packet_size.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.maximum_packet_size = Some(data);
                    }
                    _ => return Err(MqttError::MalformedHeader),
                }
            }
            // Variable Byte Integer
            0x0B => {
                let (value, len) = decode_length(iter)?;
                props_len -= len;

                if props.subscription_identifer.is_some() {
                    return Err(MqttError::ProtocolViolation);
                }

                props.subscription_identifer = Some(value);
            }
            // Binary Data
            0x09 | 0x16 => {
                let len = unpack_u16(iter)?;
                let data = unpack_bytes(iter, len as usize)?;
                props_len -= size_of::<u16>() + data.len();

                match byte {
                    0x09 => {
                        if props.correlation_data.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.correlation_data = Some(data);
                    }
                    0x16 => {
                        if props.authenication_data.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.authenication_data = Some(data);
                    }
                    _ => return Err(MqttError::MalformedHeader),
                }
            }
            // UTF-8 Encoded String
            0x03 | 0x08 | 0x12 | 0x15 | 0x1A | 0x1C | 0x1F => {
                let data = unpack_string(iter)?;
                props_len -= size_of::<u16>() + data.len();

                match byte {
                    0x03 => {
                        if props.content_type.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.content_type = Some(data);
                    }
                    0x08 => {
                        if props.response_topic.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.response_topic = Some(data);
                    }
                    0x12 => {
                        if props.assigned_client_identifier.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.assigned_client_identifier = Some(data);
                    }
                    0x15 => {
                        if props.authentication_method.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.authentication_method = Some(data);
                    }
                    0x1A => {
                        if props.response_infomation.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.response_infomation = Some(data);
                    }
                    0x1C => {
                        if props.server_reference.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.server_reference = Some(data);
                    }
                    0x1F => {
                        if props.reason_string.is_some() {
                            return Err(MqttError::ProtocolViolation);
                        }
                        props.reason_string = Some(data);
                    }
                    _ => return Err(MqttError::MalformedHeader),
                }
            }
            // UTF-8 String Pair
            0x26 => {
                let key = unpack_string(iter)?;
                let value = unpack_string(iter)?;
                props_len -= (size_of::<u16>() * 2) + value.len() + key.len();

                if let Some(property) = props.user_property.as_mut() {
                    property.push((key, value));
                } else {
                    props.user_property = Some(vec![(key, value)]);
                }
            }
            _ => return Err(MqttError::MalformedHeader),
        }
    }

    Ok(props)
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::packets::utils::unpack_u32;

    use super::{decode_length, encode_length, unpack_string, unpack_u16};

    #[test]
    fn test_encode_single_byte() {
        let len: usize = 0x1e;
        let mut bytes = BytesMut::new();
        encode_length(len, &mut bytes);
        let result = bytes.freeze();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 30);
    }

    #[test]
    fn test_encode_two_bytes() {
        let len: usize = 321;

        let mut bytes = BytesMut::new();

        encode_length(len, &mut bytes);

        let result = bytes.freeze();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 193);
        assert_eq!(result[1], 2);
    }

    #[test]
    fn test_decode_single_byte() {
        let byte: [u8; 1] = [0x1e];

        let mut iter = byte.iter();

        let (value, len) = decode_length(&mut iter).expect("Failed to decode");

        assert_eq!(value, 30);
        assert_eq!(len, 1);
    }

    #[test]
    fn test_decode_two_bytes() {
        let bytes: [u8; 2] = [193, 2];

        let mut iter = bytes.iter();

        let (value, len) = decode_length(&mut iter).unwrap();
        assert_eq!(value, 321);
        assert_eq!(len, 2);
    }

    #[test]
    fn test_decode_two_bytes_with_extra() {
        let bytes: [u8; 3] = [193, 2, 123];

        let mut iter = bytes.iter();

        let (value, len) = decode_length(&mut iter).unwrap();
        assert_eq!(value, 321);
        assert_eq!(len, 2);
    }

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
