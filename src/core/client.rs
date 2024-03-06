use tokio::sync::mpsc::Sender;

use super::{enums::ClientEvent, session::Session};

pub struct Client {
    pub session: Session,
    pub sender: Sender<ClientEvent>,
}

impl Client {
    pub fn new(sender: Sender<ClientEvent>) -> Self {
        Self {
            session: Session::new(),
            sender,
        }
    }

    pub fn clear_session(&mut self) {
        self.session.clear();
    }
}
