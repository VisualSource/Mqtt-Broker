use crate::headers::enums::{PacketType, QosLevel};
/// The remaining bits [3-0] of byte 1 in the fixed header contain flags specific to each MQTT Control Packet type
/// as listed in the [Table 2.2](http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/errata01/os/mqtt-v3.1.1-errata01-os-complete.html#_Toc442180833) - Flag Bits below
///
///   7-4         3   2-1   0  
/// Packet Type  DUP  QOS   RETAIN
pub struct MqttFixedHeader(u8);

impl MqttFixedHeader {
    /// dup:    Duplicate delivery of a PUBLISH Control Packet
    /// qos:    PUBLISH Quality of Service
    /// retain: PUBLISH Retain flag
    pub fn new(packet_type: PacketType, dup: bool, qos: QosLevel, retain: bool) -> Self {
        let mut bit: u8 = 0x00;

        // bit 0
        bit |= (retain as u8) << 0;

        // bit 3-1
        bit |= (qos as u8) << 1;

        // bit 3
        bit |= (dup as u8) << 3;

        // bit 7-4
        bit |= (packet_type as u8) << 4;

        Self(bit)
    }

    pub fn default() -> Self {
        let mut bit: u8 = 0x00;

        bit |= (true as u8) << 0;
        bit |= (QosLevel::ExactlyOnce as u8) << 1;
        bit |= (true as u8) << 3;
        bit |= (PacketType::PUBACK as u8) << 4;

        Self(bit)
    }

    pub fn set_retain(&mut self, value: bool) {
        self.0 &= !(1 << 0);
        self.0 |= (value as u8) << 0;
    }
    pub fn get_retain(&self) -> bool {
        ((self.0 & 0x01) >> 0) == 1
    }

    pub fn set_qos(&mut self, value: QosLevel) {
        self.0 &= !(2 << 1);
        self.0 |= (value as u8) << 1;
    }
    pub fn get_qos(&self) -> Result<QosLevel, ()> {
        let data = (self.0 & 0x06) >> 1; // get bytes 2-1
        QosLevel::try_from(data)
    }

    pub fn set_dup(&mut self, value: bool) {
        self.0 &= !(1 << 3);
        self.0 |= (value as u8) << 3;
    }
    pub fn get_dup(&self) -> bool {
        (self.0 & 0x08) >> 3 == 1
    }

    pub fn set_type(&mut self, value: PacketType) {
        self.0 &= !(0xF0 << 4);
        self.0 |= (value as u8) << 4;
    }
    pub fn get_type(&self) -> Result<PacketType, ()> {
        let data = (self.0 & 0xF0) >> 4;
        PacketType::try_from(data)
    }
}

impl From<&u8> for MqttFixedHeader {
    fn from(value: &u8) -> Self {
        Self(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_byte_retain_default() {
        let header = MqttFixedHeader::default();

        assert_eq!(header.get_retain(), true);
    }

    #[test]
    fn test_header_byte_retain_set() {
        let mut header = MqttFixedHeader::default();

        header.set_retain(false);

        assert_eq!(header.get_retain(), false);
    }

    #[test]
    fn test_header_byte_qos_get() {
        let header = MqttFixedHeader::default();

        assert_eq!(header.get_qos().unwrap(), QosLevel::ExactlyOnce);
    }

    #[test]
    fn test_header_byte_qos_set() {
        let mut header = MqttFixedHeader::default();

        header.set_qos(QosLevel::AtLeastOnce);

        assert_eq!(header.get_qos().unwrap(), QosLevel::AtLeastOnce);
    }

    #[test]
    fn test_header_byte_dup_get() {
        let header = MqttFixedHeader::default();
        assert_eq!(header.get_dup(), true);
    }

    #[test]
    fn test_header_byte_dup_set() {
        let mut header = MqttFixedHeader::default();

        header.set_dup(false);

        assert_eq!(header.get_dup(), false);
    }

    #[test]
    fn test_header_byte_ptype_get() {
        let header = MqttFixedHeader::default();
        assert_eq!(header.get_type().unwrap(), PacketType::PUBACK);
    }

    #[test]
    fn test_header_byte_ptype_set() {
        let mut header = MqttFixedHeader::default();

        header.set_type(PacketType::DISCONNECT);

        assert_eq!(header.get_type().unwrap(), PacketType::DISCONNECT);
    }
}
