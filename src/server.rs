#[derive(Debug)]
pub enum Event {
    Message(Vec<u8>),
    Disconnect,
}

#[derive(Debug)]
pub enum Request {
    Publish(String, Vec<u8>),
    DisconnectClient(String),
}
