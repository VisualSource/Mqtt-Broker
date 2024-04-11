use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use super::{enums::ClientEvent, session::Session};

pub struct Client {
    pub id: u128,
    pub session: Session,
    pub sender: Sender<ClientEvent>,
}

impl Client {
    pub fn new(sender: Sender<ClientEvent>) -> Self {
        let id = Uuid::new_v4().as_u128();

        Self {
            id,
            session: Session::new(),
            sender,
        }
    }

    pub fn clear_session(&mut self) {
        self.session.clear();
    }
}
