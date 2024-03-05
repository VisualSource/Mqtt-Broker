use bytes::Bytes;

#[derive(Debug)]
pub enum Event {
    Message(Vec<u8>),
    Disconnect,
}

#[derive(Debug)]
pub enum Request {
    Publish(String, Bytes),
    DisconnectClient(String),
    Exit,
}
