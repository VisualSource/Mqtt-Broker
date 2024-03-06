use bytes::Bytes;

/// Enum for handling messages sent to the message listener
#[derive(Debug)]
pub enum Command {
    Publish(String, Bytes),
    DisconnectClient(String),
    Exit,
}

#[derive(Debug)]
pub enum ClientEvent {
    Message(Bytes),
    Disconnect,
}
#[derive(Debug)]
pub enum ProtocalVersion {
    Four,
    Five,
    Unknown,
}
