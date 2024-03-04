use crate::error::MqttError;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PacketType {
    /// Client request to connect to Server
    Connect = 1,
    /// Connect acknowledgment
    Connack = 2,
    /// Publish message
    Publish = 3,
    /// Publish acknowledgment
    Puback = 4,
    /// Publish received (assured delivery part 1)
    Pubrec = 5,
    /// Publish release (assured delivery part 2)
    Pubrel = 6,
    /// Publish complete (assured delivery part 3)
    Pubcomp = 7,
    /// Client subscribe request
    Subscribe = 8,
    /// Subscribe acknowledgment
    Suback = 9,
    /// Unsubscribe request
    Unsubscribe = 10,
    /// Unsubscribe acknowledgment
    Unsuback = 11,
    /// PING request
    PingReq = 12,
    /// PING response
    PingResp = 13,
    /// Client is disconnecting
    Disconnect = 14,
    /// Authentication exchange
    /// Note: this is forbidden in a MQTT v3.1.1 context.
    Auth = 15,
}

impl TryFrom<u8> for PacketType {
    type Error = MqttError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1..=14 => unsafe { Ok(std::mem::transmute(value)) },
            _ => Err(MqttError::Convertion(
                value.to_string(),
                "PacketType".into(),
            )),
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum QosLevel {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl TryFrom<u8> for QosLevel {
    type Error = MqttError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QosLevel::AtMostOnce),
            1 => Ok(QosLevel::AtLeastOnce),
            2 => Ok(QosLevel::ExactlyOnce),
            _ => Err(MqttError::Convertion(value.to_string(), "QosLevel".into())),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ConnectReturnCode {
    /// Connection accepted
    Accepted,
    /// The Server does not support the level of the MQTT protocol requested by the Client
    UnacceptableProtocal,
    /// The Client identifier is correct UTF-8 but not allowed by the Server
    IdentifierRejected,
    /// The Network Connection has been made but the MQTT service is unavailable
    ServerUnavailable,
    /// The data in the user name or password is malformed
    BadUserOrPassword,
    /// The Client is not authorized to connect
    NotAuthorized,
}

impl TryFrom<u8> for ConnectReturnCode {
    type Error = MqttError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        ConnectReturnCode::try_from(&value)
    }
}

impl TryFrom<&u8> for ConnectReturnCode {
    type Error = MqttError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ConnectReturnCode::Accepted),
            1 => Ok(ConnectReturnCode::UnacceptableProtocal),
            2 => Ok(ConnectReturnCode::IdentifierRejected),
            3 => Ok(ConnectReturnCode::ServerUnavailable),
            4 => Ok(ConnectReturnCode::BadUserOrPassword),
            5 => Ok(ConnectReturnCode::NotAuthorized),
            _ => Err(MqttError::Convertion(
                value.to_string(),
                "ConnectReturnCode".into(),
            )),
        }
    }
}

impl Default for ConnectReturnCode {
    fn default() -> Self {
        Self::Accepted
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum SubackReturnCode {
    SuccessQosZero = 0x00,
    SuccessQosOne = 0x01,
    SuccessQosTwo = 0x02,
    Failure = 0x80,
}
impl SubackReturnCode {
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::SuccessQosZero => 0x00,
            Self::SuccessQosOne => 0x01,
            Self::SuccessQosTwo => 0x02,
            Self::Failure => 0x80,
        }
    }
}

impl TryFrom<u8> for SubackReturnCode {
    type Error = MqttError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        SubackReturnCode::try_from(&value)
    }
}

impl TryFrom<&u8> for SubackReturnCode {
    type Error = MqttError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::SuccessQosZero),
            0x01 => Ok(Self::SuccessQosOne),
            0x02 => Ok(Self::SuccessQosTwo),
            0x80 => Ok(Self::Failure),
            _ => Err(MqttError::Convertion(
                value.to_string(),
                "ReturnCode".into(),
            )),
        }
    }
}
