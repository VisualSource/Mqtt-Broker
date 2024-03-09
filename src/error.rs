use thiserror::Error;
use tokio::task::JoinError;

use crate::core::enums::Command;

#[derive(Debug, Error)]
pub enum MqttError {
    #[error("Unknown Error")]
    Unknown,

    #[error("Malformed UTF-8 String: {0}")]
    MalformedString(#[from] std::string::FromUtf8Error),

    #[error("Packet type is reserved")]
    ReservedPacketType,

    #[error("Packet is missing a required byte: {0}")]
    RequiredByteMissing(&'static str),

    #[error("Io Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol Violoation")]
    ProtocolViolation,

    #[error("Unaccptable Protocal Level")]
    UnacceptableProtocolLevel,

    #[error("Unknown protocal name")]
    UnknownProtocol,

    #[error("MalformedHeader")]
    MalformedHeader,

    #[error("Failed to send message: {0}")]
    ChannelError(#[from] tokio::sync::mpsc::error::SendError<Command>),
    #[error("Failed to convert to `{0}` to `{1}`.")]
    Convertion(String, String),
    #[error("Byte Error")]
    MissingByte,
    #[error("Malformed Remaing Length header")]
    MalformedRemaingLength,

    #[error("Missing Fixed Header")]
    MissingFixedHeader,

    #[error("Client identifier is invalid")]
    ClientIdentifierRejected,
    #[error("Failed to get client id")]
    FailedToGetCId,
    #[error("PoisonError")]
    QueuePoisonError,
    #[error("Failed to join task")]
    TaskJoinError(#[from] JoinError),
    #[error("RwLock error")]
    RwLockError,
}
