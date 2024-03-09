use bytes::{BufMut, BytesMut};

use crate::{
    error::MqttError,
    packets::{
        enums::{PacketType, QosLevel},
        traits::FromBytes,
        utils::decode_length,
    },
};

/// ### MQTT Fixed Header
/// Each MQTT Control Packet contains a fixed header.
///
/// Fixed Header format
///
/// <table>
///     <thead>
///         <tr>
///             <th>Bit</th>
///             <th>7</th>
///             <th>6</th>
///             <th>5</th>
///             <th>4</th>
///             <th>3</th>
///             <th>2</th>
///             <th>1</th>
///             <th>0</th>
///         </tr>
///     </thead>
///     <tbody>
///        <tr>
///           <th>byte 1</th>
///           <td colspan="4">MQTT Control Packet type</td>
///           <td colspan="4">Flags specific to each MQTT Control Packet type</td>
///        </tr>
///        <tr>
///             <th>byte 1</th>
///             <td colspan="8">Remaining Length</td>
///         </tr>
///     </tbody>
/// </table>
///
/// [(MQTT 3.1.1) Fixed Header](http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/errata01/os/mqtt-v3.1.1-errata01-os-complete.html#_Toc442180841)
#[derive(Debug)]
pub struct FixedHeader {
    /// ### Header Flags
    /// MQTT Control Packet type
    ///
    /// See [`PacketType`] for Control packet type
    ///
    ///
    flags: u8,
    /// ### Remaining Length
    /// Position: starts at byte 2.
    ///
    /// The Remaining Length is the number of bytes remaining within the current packet,
    /// including data in the variable header and the payload.
    /// The Remaining Length does not include the bytes used to encode the Remaining Length.
    ///
    /// The Remaining Length is encoded using a variable length encoding scheme which uses a single byte for values up to 127.
    /// Larger values are handled as follows. The least significant seven bits of each byte encode the data,
    /// and the most significant bit is used to indicate that there are following bytes in the representation.
    /// Thus each byte encodes 128 values and a "continuation bit". The maximum number of bytes in the Remaining Length field is four.
    /// [MQTT 3.1.1](http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/errata01/os/mqtt-v3.1.1-errata01-os-complete.html#_Toc442180836)
    /// [MQTT 5](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901024)
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
        bit |= retain as u8;

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

    pub fn as_byte(&self, buf: &mut BytesMut) {
        buf.put_u8(self.flags);
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
        self.flags |= value as u8
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
        self.flags & 0x01 == 1
    }
    pub fn get_remaing_len(&self) -> usize {
        self.remaining_len
    }
    pub fn set_remain_len(&mut self, value: usize) {
        self.remaining_len = value;
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
        bit |= true as u8;

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
        let mut header = FixedHeader::from(
            *iter
                .next()
                .ok_or_else(|| MqttError::RequiredByteMissing("Missing Fixed header byte"))?,
        );

        let len = decode_length(iter)?.0;
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
        assert!(!header.get_dup());
        assert!(!header.get_retain());
    }

    #[test]
    fn test_header_byte_retain_default() {
        let header = FixedHeader::default();

        assert!(header.get_retain());
    }

    #[test]
    fn test_header_byte_retain_set() {
        let mut header = FixedHeader::default();

        header.set_retain(false);

        assert!(!header.get_retain());
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
        assert!(header.get_dup());
    }

    #[test]
    fn test_header_byte_dup_set() {
        let mut header = FixedHeader::default();

        header.set_dup(false);

        assert!(!header.get_dup());
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
}
