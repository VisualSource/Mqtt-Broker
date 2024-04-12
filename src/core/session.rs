use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use super::enums::ClientEvent;

pub struct Session {
    pub id: u128,
    pub bridge: Sender<ClientEvent>,
}

impl Session {
    pub fn new(bridge: Sender<ClientEvent>) -> Self {
        let id = Uuid::new_v4().as_u128();

        Self { id, bridge }
    }
}
