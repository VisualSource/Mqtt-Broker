use crate::{
    error::MqttError,
    packets::{
        enums::QosLevel,
        traits::{FromBytes, ToBytes},
        utils::{unpack_string, unpack_u16},
    },
};

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

    pub fn as_byte(&self) -> u8 {
        self.0
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

///
#[derive(Debug, Default)]
pub struct ConnectHeader {
    pub flags: Flags,
    /// The Keep Alive is a Two Byte Integer which is a time interval measured in seconds. It is the maximum time interval that is permitted to elapse between the point at which the Client finishes transmitting one MQTT Control Packet and the point it starts sending the next.
    /// It is the responsibility of the Client to ensure that the interval between MQTT Control Packets being sent does not exceed the Keep Alive value.
    pub keepalive: u16,

    pub client_id: String,
    pub username: String,
    pub password: String,
    pub will_topic: String,
    pub will_message: String,
    pub protocal_version: u8,
}

impl ConnectHeader {
    pub fn new(
        flags: Flags,
        keepalive: u16,
        client_id: String,
        username: Option<String>,
        password: Option<String>,
        will_topic: Option<String>,
        will_message: Option<String>,
    ) -> Self {
        Self {
            flags,
            keepalive,
            client_id,
            protocal_version: 4,
            username: username.unwrap_or_default(),
            password: password.unwrap_or_default(),
            will_topic: will_topic.unwrap_or_default(),
            will_message: will_message.unwrap_or_default(),
        }
    }
}

impl FromBytes for ConnectHeader {
    type Output = ConnectHeader;

    fn from_bytes<'a, I>(
        iter: &mut I,
        _: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let mut connect_header = ConnectHeader::default();

        let protocal_name = unpack_string(iter)?;

        if &protocal_name != "MQTT" {
            return Err(MqttError::UnsupportedProtocolVersion);
        }

        connect_header.protocal_version = *iter.next().ok_or_else(|| MqttError::MissingByte)?;

        if connect_header.protocal_version != 4 || connect_header.protocal_version != 5 {
            return Err(MqttError::UnsupportedProtocolVersion);
        }

        connect_header.flags = Flags::from(iter.next().ok_or_else(|| MqttError::MissingByte)?);
        connect_header.keepalive = unpack_u16(iter)?;

        if connect_header.protocal_version == 5 {
            // Connect Properties
            // Property length

            // Session Expiry Interval
            // Receive Maximum
            // Maximum Packet Size
            // Topic Alias Maximum
            // Request Response Information
            // Request Problem Information
            // User Property
            // Authentication Method
            // Authentication Data
        }

        connect_header.client_id = unpack_string(iter)?;

        // The Server MUST allow ClientIds which are between 1 and 23 UTF-8 encoded bytes in length, and that contain only the characters
        // "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
        if connect_header.client_id.len() == 0 {
            if !connect_header.flags.clean_session() {
                return Err(MqttError::ClientIdentifierRejected);
            }

            connect_header.client_id = uuid::Uuid::new_v4().to_string();
        }

        if connect_header.flags.will() {
            if connect_header.protocal_version == 5 {
                todo!("Implement Variable string header");

                // Will Delay interval
                // Payload Format undicator
                // Message Expiry Interval
                // Content Type
                // Response Topic
                // Correlation Data
                // User Property
            }

            connect_header.will_topic = unpack_string(iter)?;
            connect_header.will_message = unpack_string(iter)?;
        }

        if connect_header.flags.has_username() {
            connect_header.username = unpack_string(iter)?;
        }

        if connect_header.flags.has_password() {
            connect_header.password = unpack_string(iter)?;
        }

        Ok(connect_header)
    }
}

impl ToBytes for ConnectHeader {
    fn to_bytes(&self) -> Result<Vec<u8>, MqttError> {
        let mut body = vec![
            vec![
                0x00,
                0x04,
                0x4d,
                0x51,
                0x54,
                0x54, // Protocal
                0x04, // Version
                self.flags.as_byte(),
            ],
            self.keepalive.to_be_bytes().to_vec(),
            (self.client_id.len() as u16).to_be_bytes().to_vec(),
            self.client_id.as_bytes().to_vec(),
        ];

        if self.flags.will() {
            body.push((self.will_topic.len() as u16).to_be_bytes().to_vec());
            body.push(self.will_topic.as_bytes().to_vec());

            body.push((self.will_message.len() as u16).to_be_bytes().to_vec());
            body.push(self.will_message.as_bytes().to_vec());
        }

        if self.flags.has_username() {
            body.push((self.username.len() as u16).to_be_bytes().to_vec());
            body.push(self.username.as_bytes().to_vec());
        }

        if self.flags.has_password() {
            body.push((self.password.len() as u16).to_be_bytes().to_vec());
            body.push(self.password.as_bytes().to_vec());
        }

        Ok(body.concat())
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
        assert_eq!(flags.will_qos().expect("QOS"), QosLevel::AtMostOnce);
        assert!(!flags.will_retain());
    }
}
