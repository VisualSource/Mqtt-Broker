use std::fmt::Display;

use crate::error::MqttError;

/// ### MQTT Control Packet type
/// Represented as a 4-bit unsigned value, the values are shown below.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PacketType {
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | CONNECT | 1 | Client to Server | Client request to connect to server |
    Connect = 1,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | CONNACK | 2 | Server to Client | Connect acknowledgment |
    Connack = 2,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | PUBLISH | 3 | Client to Server or Server to Client | Publish message |
    Publish = 3,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | PUBACK | 4 | Client to Server or Server to Client | Publish acknowledgment |
    Puback = 4,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | PUBREC | 5 | Client to Server or Server to Client | Publish received (assured delivery part 1) |
    Pubrec = 5,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | PUBREL| 6 | Client to Server or Server to Client | Publish release (assured delivery part 2) |
    Pubrel = 6,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | PUBCOMP| 7 | Client to Server or Server to Client | Publish complete (assured delivery part 3) |
    Pubcomp = 7,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | SUBSCRIBE| 8 | Client to Server  | Client subscribe request |
    Subscribe = 8,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | SUBACK| 9 | Server to Client  | Subscribe acknowledgment |
    Suback = 9,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | UNSUBSCRIBE| 10 | Client to Server  | Unsubscribe request |
    Unsubscribe = 10,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | UNSUBACK| 11 | Server to Client  | Unsubscribe acknowledgment |
    Unsuback = 11,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | UNSUBACK| 12 | Client to Server  | PING request |
    PingReq = 12,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | PINGRESP | 13 | Server to Client  | PING response |
    PingResp = 13,
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----:            | :---------: |
    /// | DISCONNECT | 14 | Client to Server  | Client is disconnecting |
    Disconnect = 14,
    /// MQTT v3.1.1
    ///
    /// | Name | Value | Direction of flow | Description |
    /// | :--: |:----: |:-----------------: | :---------: |
    /// | Reserved | 15 | Forbidden  | Reserved |
    ///
    /// MQTT v5
    ///
    /// | Name | Value | Direction of flow | Description |
    /// | :---: |:----: | :---------------: | :---------: |
    /// | AUTH | 15 | Client to Server or Server to Client  | Authentication exchange |
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

/// ### Quality of Service levels
///
/// MQTT delivers Application Messages according to the Quality of Service (QoS) levels defined here.
/// The delivery protocol is symmetric, in the description below the Client and Server can each take the role of either Sender or Receiver.
/// The delivery protocol is concerned solely with the delivery of an application message from a single Sender to a single Receiver.
/// When the Server is delivering an Application Message to more than one Client, each Client is treated independently.
/// The QoS level used to deliver an Application Message outbound to the Client could differ from that of the inbound Application Message.
/// The non-normative flow diagrams in the following sections are intended to show possible implementation approaches.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum QosLevel {
    /// #### QoS 0: At most once delivery
    ///
    /// The message is delivered according to the capabilities of the underlying network.
    /// No response is sent by the receiver and no retry is performed by the sender.
    /// The message arrives at the receiver either once or not at all.
    AtMost,
    ///  #### QoS 1: At least once delivery
    ///
    /// This quality of service ensures that the message arrives at the receiver at least once.
    /// A QoS 1 PUBLISH Packet has a Packet Identifier in its variable header and is acknowledged by a PUBACK Packet.
    /// Section 2.3.1 provides more information about Packet Identifiers.
    AtLeast,
    /// #### QoS 2: Exactly once delivery
    ///
    /// This is the highest quality of service, for use when neither loss nor duplication of messages are acceptable. There is an increased overhead associated with this quality of service.
    Exactly,
}

impl From<QosLevel> for u8 {
    fn from(value: QosLevel) -> Self {
        value as u8
    }
}

impl Display for QosLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QosLevel::AtMost => write!(f, "Qos 0 (At Most Once)"),
            QosLevel::AtLeast => write!(f, "Qos 1 (At Least Once)"),
            QosLevel::Exactly => write!(f, "Qos 2 (Ecactly Once)"),
        }
    }
}

impl TryFrom<u8> for QosLevel {
    type Error = MqttError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QosLevel::AtMost),
            1 => Ok(QosLevel::AtLeast),
            2 => Ok(QosLevel::Exactly),
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
    V4UnacceptableProtocal,
    /// The Client identifier is correct UTF-8 but not allowed by the Server
    V4IdentifierRejected,
    /// The Network Connection has been made but the MQTT service is unavailable
    V4ServerUnavailable,
    /// The data in the user name or password is malformed
    V4BadUserNameOrPassword,
    /// The Client is not authorized to connect
    V4NotAuthorized,

    /// The Server does not wish to reveal the reason for the failure, or none of the other Reason Codes apply.
    UnspecifiedError = 0x80,
    /// Data within the CONNECT packet could not be correctly parsed.
    MalformedPacket = 0x81,
    /// Data in the CONNECT packet does not conform to this specification.
    ProtocolError = 0x82,

    /// The CONNECT is valid but is not accepted by this Server.
    ImplementationSpecificError = 0x83,
    /// The Server does not support the version of the MQTT protocol requested by the Client.
    UnsupportedProtocolVersion = 0x84,
    /// The Client Identifier is a valid string but is not allowed by the Server.
    ClientIdentifierNotValid = 0x85,
    /// The Server does not accept the User Name or Password specified by the Client
    BadUserNameOrPassword = 0x86,
    /// The Client is not authorized to connect.
    V5NotAuthorized = 0x87,
    /// The MQTT Server is not available.
    V5ServerUnavailable = 0x88,
    /// The Server is busy. Try again later.
    ServerBusy = 0x89,
    /// This Client has been banned by administrative action. Contact the server administrator.
    Banned = 0x8A,
    /// The authentication method is not supported or does not match the authentication method currently in use.
    BadAuthenticationMethod = 0x8C,
    /// The Will Topic Name is not malformed, but is not accepted by this Server.
    TopicNameInvalid = 0x90,
    /// The CONNECT packet exceeded the maximum permissible size.
    PacketTooLarge = 0x95,
    /// An implementation or administrative imposed limit has been exceeded.
    QuotaExceeded = 0x97,
    /// The Will Payload does not match the specified Payload Format Indicator.
    PayloadFormatInvalid = 0x99,
    /// The Server does not support retained messages, and Will Retain was set to 1.
    RetainNotSupported = 0x9A,
    /// The Server does not support the QoS set in Will QoS.
    QoSNotSupported = 0x9B,
    /// The Client should temporarily use another server.
    UseAnotherServer = 0x9C,
    /// The Client should permanently use another server.
    ServerMoved = 0x9D,
    /// The connection rate limit has been exceeded.
    ConnectionRateExceeded = 0x9F,
}

impl From<ConnectReturnCode> for u8 {
    fn from(value: ConnectReturnCode) -> Self {
        value as u8
    }
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
            1 => Ok(ConnectReturnCode::V4UnacceptableProtocal),
            2 => Ok(ConnectReturnCode::V4IdentifierRejected),
            3 => Ok(ConnectReturnCode::V4ServerUnavailable),
            4 => Ok(ConnectReturnCode::V4BadUserNameOrPassword),
            5 => Ok(ConnectReturnCode::V4NotAuthorized),
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

impl From<SubackReturnCode> for u8 {
    fn from(value: SubackReturnCode) -> Self {
        value as u8
    }
}

impl SubackReturnCode {
    #[deprecated]
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
