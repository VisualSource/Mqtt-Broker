use std::{collections::HashMap, sync::RwLock};

use log::error;
use tokio::sync::mpsc::Sender;

use crate::{error::MqttError, packets::enums::QosLevel, server::Event};

use self::{client::Client, enums::ClientEvent, topic::Topics};

mod client;
pub mod enums;
pub mod info;
mod pattern;
mod session;
mod topic;

pub struct App {
    pub clients: RwLock<HashMap<String, Client>>,
    pub topics: RwLock<Topics>,
}

impl App {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            topics: RwLock::new(Topics::new()),
        }
    }

    pub async fn disconnect_client(&self, cid: &String) -> Result<(), MqttError> {
        let r = self.clients.read().map_err(|_| MqttError::RwLockError)?;

        let client = r.get(cid).ok_or_else(|| MqttError::RwLockError)?;

        if let Err(e) = client.sender.send(ClientEvent::Disconnect).await {
            error!("{}", e);
        }

        Ok(())
    }

    pub fn clear_user_session(&self, cid: &String) -> Result<(), MqttError> {
        let mut c = self.clients.write().map_err(|_| MqttError::RwLockError)?;

        let user = c.get_mut(cid).ok_or_else(|| MqttError::RwLockError)?;

        user.clear_session();

        Ok(())
    }

    pub fn has_client(&self, cid: &String) -> Result<bool, MqttError> {
        let c = self.clients.read().map_err(|_| MqttError::RwLockError)?;

        Ok(c.contains_key(cid))
    }

    pub fn add_client(&self, cid: String, tx: Sender<ClientEvent>) -> Result<(), MqttError> {
        let client = Client::new(tx);

        let mut data = self.clients.write().map_err(|_| MqttError::RwLockError)?;

        data.insert(cid, client);

        Ok(())
    }

    pub fn remove_client(&self, cid: &String) -> Result<(), MqttError> {
        let mut c = self.clients.write().map_err(|_| MqttError::RwLockError)?;
        let mut t = self.topics.write().map_err(|_| MqttError::RwLockError)?;
        let client = c.get(cid).ok_or_else(|| MqttError::RwLockError)?;

        for sub in &client.session.subscriptions {
            t.unsubscribe(cid, &sub.0);
        }

        c.remove(cid);

        Ok(())
    }

    pub fn add_subscriber_to_topic(
        &self,
        topic_name: String,
        cid: &String,
        qos: QosLevel,
        tx: Sender<ClientEvent>,
    ) -> Result<(), MqttError> {
        let mut t = self.topics.write().map_err(|_| MqttError::RwLockError)?;

        t.subscribe(topic_name, qos, cid, tx)
            .map_err(|_| MqttError::RwLockError)?;

        Ok(())
    }

    pub fn remove_subscriber_from_topic(
        &self,
        topic_name: &String,
        cid: &String,
    ) -> Result<(), MqttError> {
        let mut t = self.topics.write().map_err(|_| MqttError::RwLockError)?;
        t.unsubscribe(cid, topic_name);
        Ok(())
    }
}

#[cfg(test)]
mod test {}
