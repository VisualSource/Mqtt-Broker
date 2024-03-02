use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MqttError {
    #[error("Failed to parse header")]
    MalformedHeader,
    #[error("Failed to read byte: {0}")]
    ByteRead(#[from] std::io::Error),

    #[error("Failed to parse a u16")]
    MalformedU16,

    #[error("Failed to parse u32")]
    MalformedU32,

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
}
