use crate::utils::FromBytes;

use super::fixed_header::MqttFixedHeader;

#[repr(u8)]
pub enum ConnectReturnCode {
    Accepted,
    RefusedBadProtocol,
    RefusedIdentReject,
    RefusedServerUnavaliable,
    RefusedBadCred,
    RefusedUnauthorized,
}

pub struct AcknowledgeFlags(u8);

impl AcknowledgeFlags {
    pub fn session_present(&self) -> bool {
        (self.0 & 0x01) >> 0 == 1
    }
    pub fn set_session_present(&mut self, value: bool) {
        self.0 = value as u8;
    }
}

pub struct MqttConnack {
    header: MqttFixedHeader,
    acknowledge_flags: AcknowledgeFlags,
    rc: ConnectReturnCode,
}

impl FromBytes for MqttConnack {
    type Output = MqttConnack;

    fn from_bytes<'a, I>(iter: I, header: MqttFixedHeader) -> Result<Self::Output, ()>
    where
        I: Iterator<Item = &'a u8>,
    {
        todo!()
    }
}
