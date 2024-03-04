use tokio::sync::mpsc::Sender;

use crate::server::Event;

use super::session::Session;

pub struct Client {
    pub session: Session,
    pub sender: Sender<Event>,
}

impl Client {
    pub fn new(sender: Sender<Event>) -> Self {
        Self {
            session: Session::new(),
            sender,
        }
    }

    pub fn clear_session(&mut self) {
        self.session.clear();
    }
}
