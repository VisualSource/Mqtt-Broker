use std::string::FromUtf8Error;

use thiserror::Error;

use crate::server::Request;

#[derive(Debug, Error)]
pub enum MqttError {
    #[error("Failed to parse header")]
    MalformedHeader,
    #[error("Io Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse a u16")]
    MalformedU16,

    #[error("Failed to parse u32")]
    MalformedU32,

    #[error("Failed to send message: {0}")]
    ChannelError(#[from] tokio::sync::mpsc::error::SendError<Request>),

    #[error("Failed to convert to `{0}` to `{1}`.")]
    Convertion(String, String),
    #[error("Failed to parse utf8 string")]
    Utf8Error(#[from] FromUtf8Error),
    #[error("Failed to obtain required byte")]
    MissingByte,
    #[error("Invalid length")]
    InvalidLength,
    #[error("Malformed Remaing Length header")]
    MalformedRemaingLength,

    #[error("Insupported Protocol Version")]
    UnsupportedProtocolVersion,

    #[error("Invalid Packet Type")]
    InvalidPacketType,

    #[error("Missing Fixed Header")]
    MissingFixedHeader,
    #[error("Protocol Violoation")]
    ProtocolViolation,

    #[error("Client identifier is invalid")]
    ClientIdentifierRejected,

    #[error("Failed to get client id")]
    FailedToGetCId,
}
