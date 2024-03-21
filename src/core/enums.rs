use bytes::Bytes;

use crate::packets::enums::QosLevel;
use crate::{error::MqttError, packets::enums::SubackReturnCode};

pub type Responder<T> = tokio::sync::oneshot::Sender<T>;
pub type Receiver<T> = tokio::sync::mpsc::Sender<T>;

/// Enum for handling messages sent to the message listener
#[derive(Debug)]
pub enum Command {
    RegisterClient {
        id: String,
        message_channel: Receiver<ClientEvent>,
        protocol: ProtocalVersion,
        clean_session: bool,
        callback: Responder<Result<(), MqttError>>,
    },

    Subscribe {
        client: String,
        topics: Vec<(String, QosLevel)>,
        callback: Responder<Result<Vec<SubackReturnCode>, MqttError>>,
    },

    Publish {
        topic: String,
        payload: Bytes,
    },
    Unsubscribe {
        topics: Vec<String>,
        client: String,
        callback: Responder<Result<(), MqttError>>,
    },
    DisconnectClient(String),
    Exit,
}

#[derive(Debug)]
pub enum ClientEvent {
    Message(Bytes),
    Disconnect,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ProtocalVersion {
    Four,
    Five,
    Unknown,
}

impl From<ProtocalVersion> for u8 {
    fn from(value: ProtocalVersion) -> Self {
        match value {
            ProtocalVersion::Five => 5,
            ProtocalVersion::Four => 4,
            _ => 0,
        }
    }
}

impl From<u8> for ProtocalVersion {
    fn from(value: u8) -> Self {
        match value {
            4 => Self::Four,
            5 => Self::Five,
            _ => Self::Unknown,
        }
    }
}
