use crate::{
    error::MqttError,
    packets::{
        enums::{PacketType, QosLevel},
        traits::FromBytes,
    },
};

const MAX_ENCODED_SIZE: usize = 128 * 128 * 128;

/// ## MQTT Control Packet format - Fixed Header
/// Contains two bytes on containing Packet type and flags
/// and the other the reaining length of the packet.
#[derive(Debug)]
pub struct FixedHeader {
    flags: u8,
    remaining_len: usize,
}

impl FixedHeader {
    /// Create a new Fixed Header
    ///
    /// # Arguments
    ///
    /// * `packet_type` - The type of packet that this is.
    /// * `dup` -  Duplicate delivery of a PUBLISH Control Packet
    /// * `qos` -  PUBLISH Quality of Service
    /// * `retain` - PUBLISH Retain flag
    /// * `len` - Remaining Length of packet.
    pub fn new(
        packet_type: PacketType,
        dup: bool,
        qos: QosLevel,
        retain: bool,
        len: usize,
    ) -> Self {
        let mut bit: u8 = 0x00;

        // bit 0
        bit |= (retain as u8) << 0;

        // bit 2-1
        bit |= (qos as u8) << 1;

        // bit 3
        bit |= (dup as u8) << 3;

        // bit 7-4
        bit |= (packet_type as u8) << 4;

        Self {
            flags: bit,
            remaining_len: len,
        }
    }

    pub fn as_byte(&self) -> u8 {
        self.flags
    }

    pub fn set_packet_type(&mut self, value: PacketType) {
        self.flags &= !(0xF0 << 4);
        self.flags |= (value as u8) << 4;
    }
    pub fn set_dup(&mut self, value: bool) {
        self.flags &= !(1 << 3);
        self.flags |= (value as u8) << 3;
    }
    pub fn set_qos(&mut self, value: QosLevel) {
        self.flags &= !(2 << 1);
        self.flags |= (value as u8) << 1;
    }
    pub fn set_retain(&mut self, value: bool) {
        self.flags &= !(1 << 0);
        self.flags |= (value as u8) << 0;
    }
    pub fn get_packet_type(&self) -> Result<PacketType, MqttError> {
        let data = (self.flags & 0xF0) >> 4;
        PacketType::try_from(data)
    }
    pub fn get_dup(&self) -> bool {
        (self.flags & 0x08) >> 3 == 1
    }
    pub fn get_qos(&self) -> Result<QosLevel, MqttError> {
        let data = (self.flags & 0x06) >> 1; // get bytes 2-1
        QosLevel::try_from(data)
    }
    pub fn get_retain(&self) -> bool {
        ((self.flags & 0x01) >> 0) == 1
    }
    pub fn get_remaing_len(&self) -> usize {
        self.remaining_len
    }
    pub fn set_remain_len(&mut self, value: usize) {
        self.remaining_len = value;
    }

    fn decode_length<'a, I>(iter: &mut I) -> Result<usize, MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let mut multiplier: usize = 1;
        let mut value = 0;

        loop {
            let byte = iter.next().ok_or_else(|| MqttError::MissingByte)?;

            value += ((byte & 127) as usize) * multiplier;

            if multiplier > MAX_ENCODED_SIZE {
                return Err(MqttError::MalformedRemaingLength);
            }

            multiplier *= 128;

            if (byte & 128) == 0 {
                break;
            }
        }

        Ok(value)
    }
}

impl From<u8> for FixedHeader {
    fn from(value: u8) -> Self {
        Self {
            flags: value,
            remaining_len: 0,
        }
    }
}

impl Default for FixedHeader {
    fn default() -> Self {
        let mut bit: u8 = 0x00;

        // bit 0
        bit |= (true as u8) << 0;

        // bit 2-1
        bit |= (QosLevel::ExactlyOnce as u8) << 1;

        // bit 3
        bit |= (true as u8) << 3;

        // bit 7-4
        bit |= (PacketType::Puback as u8) << 4;

        Self {
            flags: bit,
            remaining_len: Default::default(),
        }
    }
}

impl FromBytes for FixedHeader {
    type Output = FixedHeader;

    fn from_bytes<'a, I>(iter: &mut I, _: Option<&FixedHeader>) -> Result<Self::Output, MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let control = *iter.next().ok_or_else(|| MqttError::MissingByte)?;
        let mut header = FixedHeader::from(control);

        let len = FixedHeader::decode_length(iter)?;
        header.set_remain_len(len);

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fixed_header() {
        // Fixed Header
        // length: 0,

        let data: [u8; 2] = [0x10, 0x07];

        let mut iter = data.iter();

        let header = FixedHeader::from_bytes(&mut iter, None).expect("Failed to parse header");

        assert_eq!(
            header.get_packet_type().expect("Failed to get packet type"),
            PacketType::Connect
        );
        assert_eq!(
            header.get_qos().expect("Failed to get qos"),
            QosLevel::AtMostOnce
        );
        assert_eq!(header.get_dup(), false);
        assert_eq!(header.get_retain(), false);
    }

    #[test]
    fn test_header_byte_retain_default() {
        let header = FixedHeader::default();

        assert_eq!(header.get_retain(), true);
    }

    #[test]
    fn test_header_byte_retain_set() {
        let mut header = FixedHeader::default();

        header.set_retain(false);

        assert_eq!(header.get_retain(), false);
    }

    #[test]
    fn test_header_byte_qos_get() {
        let header = FixedHeader::default();

        assert_eq!(header.get_qos().unwrap(), QosLevel::ExactlyOnce);
    }

    #[test]
    fn test_header_byte_qos_set() {
        let mut header = FixedHeader::default();

        header.set_qos(QosLevel::AtLeastOnce);

        assert_eq!(header.get_qos().unwrap(), QosLevel::AtLeastOnce);
    }

    #[test]
    fn test_header_byte_dup_get() {
        let header = FixedHeader::default();
        assert_eq!(header.get_dup(), true);
    }

    #[test]
    fn test_header_byte_dup_set() {
        let mut header = FixedHeader::default();

        header.set_dup(false);

        assert_eq!(header.get_dup(), false);
    }

    #[test]
    fn test_header_byte_ptype_get() {
        let header = FixedHeader::default();
        assert_eq!(header.get_packet_type().unwrap(), PacketType::Puback);
    }

    #[test]
    fn test_header_byte_ptype_set() {
        let mut header = FixedHeader::default();

        header.set_packet_type(PacketType::Disconnect);

        assert_eq!(header.get_packet_type().unwrap(), PacketType::Disconnect);
    }

    #[test]
    fn test_decode_single_byte() {
        let byte: [u8; 1] = [0x1e];

        let mut iter = byte.iter();

        let value = FixedHeader::decode_length(&mut iter).expect("Failed to decode");

        assert_eq!(value, 30)
    }

    #[test]
    fn test_decode_two_bytes() {
        let bytes: [u8; 2] = [193, 2];

        let mut iter = bytes.iter();

        let value = FixedHeader::decode_length(&mut iter).unwrap();
        assert_eq!(value, 321);
    }

    #[test]
    fn test_decode_two_bytes_with_extra() {
        let bytes: [u8; 3] = [193, 2, 123];

        let mut iter = bytes.iter();

        let value = FixedHeader::decode_length(&mut iter).unwrap();
        assert_eq!(value, 321);
    }
}