use crate::utils::{unpack_string, unpack_u16, FromBytes};

use super::{enums::QosLevel, fixed_header::MqttFixedHeader};

/// ### Connect Flags
/// The Connect Flags byte contains a number of parameters specifying the behavior of the MQTT connection. It also indicates the presence or absence of fields in the payload.
pub struct Flags(u8);

impl Flags {
    pub fn clean_session(&self) -> bool {
        (self.0 & 0x1) >> 0 == 1
    }
    pub fn will_flag(&self) -> bool {
        (self.0 & 0x02) >> 1 == 1
    }
    pub fn will_qos(&self) -> Result<QosLevel, ()> {
        QosLevel::try_from((self.0 & 0xC0) >> 3)
    }
    pub fn will_retain(&self) -> bool {
        (self.0 & 0x10) >> 5 == 1
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

impl From<&u8> for Flags {
    fn from(value: &u8) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Default)]
pub struct Payload {
    pub keepalive: u16,
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub will_topic: String,
    pub will_message: String,
}

impl Payload {
    pub fn set_keepalive(&mut self, value: u16) {
        self.keepalive = value;
    }
    pub fn set_client_id(&mut self, value: String) {
        self.client_id = value;
    }
    pub fn set_username(&mut self, value: String) {
        self.username = value;
    }
    pub fn set_password(&mut self, value: String) {
        self.password = value;
    }
    pub fn set_will_topic(&mut self, value: String) {
        self.will_topic = value;
    }
    pub fn set_will_message(&mut self, value: String) {
        self.will_message = value;
    }
}

pub struct MqttConnect {
    pub mqtt_header: MqttFixedHeader,
    pub flags: Flags,
    pub payload: Payload,
}

impl FromBytes for MqttConnect {
    type Output = MqttConnect;
    fn from_bytes<'a, I>(mut iter: I, header: MqttFixedHeader) -> Result<Self::Output, ()>
    where
        I: Iterator<Item = &'a u8>,
    {
        let protocal_name = unpack_string(&mut iter)?;

        if protocal_name != "MQTT" {
            return Err(());
        }

        let protocal_level = iter.next().ok_or_else(|| ())?; // should be 4 for MQTT 3.1.1

        if protocal_level != &4 {
            return Err(());
        }

        let connect_flag_byte = iter.next().ok_or_else(|| ())?;

        let flags = Flags::from(connect_flag_byte);

        let mut payload = Payload::default();

        let keepalive = unpack_u16(&mut iter)?;
        payload.set_keepalive(keepalive);

        let client_id = unpack_string(&mut iter)?;
        if client_id.len() <= 0 {
            return Err(());
        }
        payload.set_client_id(client_id);

        if flags.will_flag() {
            let will_topic = unpack_string(&mut iter)?;
            payload.set_will_topic(will_topic);

            let will_messege = unpack_string(&mut iter)?;
            payload.set_will_message(will_messege);
        }

        if flags.has_username() {
            let username = unpack_string(&mut iter)?;

            payload.set_username(username);
        }

        if flags.has_password() {
            let password = unpack_string(&mut iter)?;
            payload.set_password(password);
        }

        Ok(MqttConnect {
            mqtt_header: header,
            flags: flags,
            payload: payload,
        })
    }
}
