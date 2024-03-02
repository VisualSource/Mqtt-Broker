use crate::{
    error::MqttError,
    packets::{
        enums::QosLevel,
        traits::{FromBytes, ToBytes},
        utils::{self, unpack_string, unpack_u16},
    },
};

/// ### Connect Flags
/// The Connect Flags byte contains a number of parameters specifying the behavior of the MQTT connection. It also indicates the presence or absence of fields in the payload.
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

    pub fn clean_session(&self) -> bool {
        (self.0 & 0x2) >> 1 == 1
    }
    pub fn will(&self) -> bool {
        (self.0 & 0x04) >> 2 == 1
    }
    pub fn will_qos(&self) -> Result<QosLevel, MqttError> {
        QosLevel::try_from((self.0 & 0x18) >> 3)
    }
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

#[derive(Debug, Default)]
pub struct ConnectHeader {
    pub flags: Flags,
    pub keepalive: u16,
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub will_topic: String,
    pub will_message: String,
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
            username: username.unwrap_or_default(),
            password: password.unwrap_or_default(),
            will_topic: will_topic.unwrap_or_default(),
            will_message: will_message.unwrap_or_default(),
        }
    }
}

impl FromBytes for ConnectHeader {
    type Output = ConnectHeader;

    fn from_byte_stream(
        iter: &mut std::io::Bytes<&mut std::net::TcpStream>,
        header: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, MqttError> {
        let mut connect_header = ConnectHeader::default();

        let protocal_name = utils::stream::unpack_string(iter)?;
        if &protocal_name != "MQTT" {
            return Err(MqttError::UnsupportedProtocolVersion);
        }

        let protocal_version = iter
            .next()
            .ok_or_else(|| MqttError::MalformedHeader)?
            .map_err(|e| MqttError::ByteRead(e))?;

        if protocal_version != 4 {
            return Err(MqttError::UnsupportedProtocolVersion);
        }

        let flags = iter
            .next()
            .ok_or_else(|| MqttError::MalformedHeader)?
            .map_err(|e| MqttError::ByteRead(e))?;

        connect_header.flags = Flags::from(flags);
        connect_header.keepalive = utils::stream::unpack_u16(iter)?;
        connect_header.client_id = utils::stream::unpack_string(iter)?;

        if connect_header.flags.will() {
            connect_header.will_topic = utils::stream::unpack_string(iter)?;
            connect_header.will_message = utils::stream::unpack_string(iter)?;
        }

        if connect_header.flags.has_username() {
            connect_header.username = utils::stream::unpack_string(iter)?;
        }

        if connect_header.flags.has_password() {
            connect_header.password = utils::stream::unpack_string(iter)?;
        }

        Ok(connect_header)
    }

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

        let protocal_version = *iter.next().ok_or_else(|| MqttError::MissingByte)?;
        // Just supporteding MQTT 3.1.1
        if protocal_version != 4 {
            return Err(MqttError::UnsupportedProtocolVersion);
        }

        connect_header.flags = Flags::from(iter.next().ok_or_else(|| MqttError::MissingByte)?);

        connect_header.keepalive = unpack_u16(iter)?;

        connect_header.client_id = unpack_string(iter)?;

        if connect_header.flags.will() {
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
