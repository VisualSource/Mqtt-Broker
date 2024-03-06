use std::collections::HashMap;

use log::error;
use tokio::sync::mpsc::Sender;

use crate::{error::MqttError, packets::enums::QosLevel};

use self::{
    client::Client,
    enums::ClientEvent,
    topic::{Subscriber, Topics},
};

pub mod broker_info;
mod client;
pub mod enums;
mod pattern;
mod session;
mod topic;

pub struct App {
    clients: HashMap<String, Client>,
    topics: Topics,
}

impl App {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            topics: Topics::new(),
        }
    }

    pub async fn disconnect_client(&self, cid: &String) {
        if let Some(client) = self.clients.get(cid) {
            if let Err(e) = client.sender.send(ClientEvent::Disconnect).await {
                error!("{}", e);
            }
        }
    }

    pub fn get_matches(&self, topic: &str) -> Vec<&Subscriber> {
        self.topics.get_matches(topic)
    }

    pub fn clear_client_session(&mut self, cid: &String) {
        if let Some(client) = self.clients.get_mut(cid) {
            client.clear_session();
        }
    }

    pub fn get_client(&self, id: &String) -> Option<&Client> {
        self.clients.get(id)
    }

    pub fn has_client(&self, cid: &String) -> bool {
        self.clients.contains_key(cid)
    }

    pub fn update_client(&mut self, id: &String, tx: Sender<ClientEvent>) {
        if let Some(client) = self.clients.get_mut(id) {
            client.sender = tx;
        } else {
            self.add_client(id.clone(), tx);
        }
    }

    pub fn add_client(&mut self, cid: String, tx: Sender<ClientEvent>) {
        let client = Client::new(tx);

        self.clients.insert(cid, client);
    }

    pub fn remove_client(&mut self, cid: &String) {
        if let Some(client) = self.clients.get(cid) {
            for sub in &client.session.subscriptions {
                self.topics.unsubscribe(cid, &sub.0);
            }

            self.clients.remove(cid);
        }
    }

    pub fn add_subscriber_to_topic(
        &mut self,
        topic_name: String,
        cid: &String,
        qos: QosLevel,
        tx: Sender<ClientEvent>,
    ) -> Result<(), MqttError> {
        self.topics
            .subscribe(topic_name, qos, cid, tx)
            .map_err(|_| MqttError::RwLockError)?;

        Ok(())
    }

    pub fn remove_subscriber_from_topic(&mut self, topic_name: &String, cid: &String) {
        self.topics.unsubscribe(cid, topic_name);
    }
}

#[cfg(test)]
mod test {}
