use crate::{
    error::MqttError,
    packets::{
        enums::ConnectReturnCode,
        traits::{FromBytes, ToBytes},
    },
};

#[derive(Debug, Default)]
pub struct AcknowledgeFlags(u8);

impl AcknowledgeFlags {
    pub fn new(session_present: bool) -> Self {
        Self(session_present as u8)
    }

    pub fn set_flags(&mut self, value: u8) {
        self.0 = value;
    }

    pub fn session_present(&self) -> bool {
        self.0 & 0x01 == 1
    }
    pub fn set_session_present(&mut self, value: bool) {
        self.0 = value as u8;
    }
    pub fn as_byte(&self) -> u8 {
        self.0
    }
}

impl From<u8> for AcknowledgeFlags {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

/// The CONNACK Packet is the packet sent by the Server in response to a CONNECT Packet received from a Client.
#[derive(Debug, Default)]
pub struct ConnackHeader {
    pub acknowledge_flags: AcknowledgeFlags,
    pub return_code: ConnectReturnCode,
}

impl ConnackHeader {
    pub fn builder() -> Self {
        Self {
            acknowledge_flags: AcknowledgeFlags::default(),
            return_code: ConnectReturnCode::ImplementationSpecificError,
        }
    }
    pub fn new(flags: AcknowledgeFlags, rc: ConnectReturnCode) -> ConnackHeader {
        Self {
            acknowledge_flags: flags,
            return_code: rc,
        }
    }
}

impl FromBytes for ConnackHeader {
    type Output = ConnackHeader;

    fn from_bytes<'a, I>(
        iter: &mut I,
        _: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, crate::error::MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let flags = iter.next().ok_or_else(|| MqttError::MissingByte)?;

        let mut connack = ConnackHeader::builder();

        connack.acknowledge_flags.set_flags(*flags);

        let rc = iter.next().ok_or_else(|| MqttError::MissingByte)?;
        connack.return_code = ConnectReturnCode::try_from(rc)?;

        // If Version 5

        // Property Length
        // Session Epiry Interval
        // Receive Maximnum
        // Maximum Qos
        // Retain Available
        // Maximum Packet Size
        // Assigned Client Identifier
        // Topic Alias Maximum
        // Reason String
        //  User Property
        //  Wildcard Subscription Available
        // Subscription Identifiers Available
        // Shared Subscription Available
        //  Server Keep Alive
        //  Response Information
        //  Server Reference
        //  Authentication Method
        // Authentication Data

        Ok(connack)
    }
}

impl ToBytes for ConnackHeader {
    fn to_bytes(&self) -> Result<Vec<u8>, MqttError> {
        Ok(vec![
            self.acknowledge_flags.as_byte(),
            (self.return_code as u8),
        ])
    }
}
