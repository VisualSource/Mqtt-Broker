use crate::{error::MqttError, packets::enums::QosLevel};

/// ### Connect Flags
/// The Connect Flags byte contains a number of parameters specifying the behavior of the MQTT connection. It also indicates the presence or absence of fields in the payload.
///
/// |Bit|  7             |       6       |     5       |    4-3     |   2     |   1         |  0       |
/// | - | - | - | - | - | - | - |  - |
/// |   | User Name Flag | Password Flag | Will Retain | Will QOS | Will Flag | Clean Start | Reserved |
#[derive(Debug, Default)]
pub struct Flags(u8);

impl Flags {
    pub fn new(
        clean_session: bool,
        will: bool,
        will_qos: QosLevel,
        will_retain: bool,
        password: bool,
        username: bool,
    ) -> Self {
        let mut bit: u8 = 0x00;
        bit |= (clean_session as u8) << 1;
        bit |= (will as u8) << 2;
        bit |= (will_qos as u8) << 3;
        bit |= (will_retain as u8) << 5;
        bit |= (password as u8) << 6;
        bit |= (username as u8) << 7;

        Self(bit)
    }

    pub fn validate_flags(&self) -> bool {
        self.0 & 0x1 == 1
    }

    /// #### Clean Start or Clean Session
    /// This bit specifies whether the Connection starts a new Session or is a continuation of an existing Session.
    pub fn clean_session(&self) -> bool {
        (self.0 & 0x2) >> 1 == 1
    }
    pub fn will(&self) -> bool {
        (self.0 & 0x04) >> 2 == 1
    }
    /// These two bits specify the QoS level to be used when publishing the Will Message.
    pub fn will_qos(&self) -> Result<QosLevel, MqttError> {
        QosLevel::try_from((self.0 & 0x18) >> 3)
    }
    /// This bit specifies if the Will Message is to be retained when it is published.
    pub fn will_retain(&self) -> bool {
        (self.0 & 0x20) >> 5 == 1
    }

    pub fn has_password(&self) -> bool {
        (self.0 & 0x40) >> 6 == 1
    }
    pub fn has_username(&self) -> bool {
        (self.0 & 0x80) >> 7 == 1
    }

    pub fn set_clean_session(&mut self, value: bool) {
        self.0 &= !(0x02 << 1);
        self.0 |= (value as u8) << 1;
    }

    pub fn set_will_flag(&mut self, value: bool) {
        self.0 &= !(0x04 << 2);
        self.0 |= (value as u8) << 2;
    }

    pub fn set_will_qos(&mut self, value: QosLevel) {
        self.0 &= !(0x18 << 3);
        self.0 |= (value as u8) << 3;
    }

    pub fn set_will_retain(&mut self, value: bool) {
        self.0 &= !(0x20 << 5);
        self.0 |= (value as u8) << 5;
    }

    pub fn set_has_password(&mut self, value: bool) {
        self.0 &= !(0x40 << 6);
        self.0 |= (value as u8) << 6;
    }

    pub fn set_has_username(&mut self, value: bool) {
        self.0 &= !(0x80 << 7);
        self.0 |= (value as u8) << 7;
    }
}

impl From<Flags> for u8 {
    fn from(value: Flags) -> Self {
        value.0
    }
}

impl From<u8> for Flags {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<&u8> for Flags {
    fn from(value: &u8) -> Self {
        Self(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use crate::packets::enums::QosLevel;

    use super::Flags;

    #[test]
    fn test_connect_header_flags_parse() {
        // clean session yes
        // will no
        // qos 0
        // retain no
        // password yes
        // username yes
        let flags = Flags(0xc2);

        assert!(flags.clean_session());
        assert!(flags.has_password());
        assert!(flags.has_username());
        assert!(!flags.will());
        assert_eq!(flags.will_qos().expect("QOS"), QosLevel::AtMost);
        assert!(!flags.will_retain());
    }
}
