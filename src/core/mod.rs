use std::collections::HashMap;

use log::error;
use tokio::sync::mpsc::Sender;

use crate::{
    error::MqttError,
    packets::enums::{QosLevel, SubackReturnCode},
    topic_heir::{SubscriptionLeaf, SubscriptionTree},
};

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
    subscriptions: SubscriptionTree,
}

impl App {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            subscriptions: SubscriptionTree::new(),
        }
    }

    pub fn subscribers(
        &self,
        topic: String,
    ) -> Result<Vec<(u128, Sender<ClientEvent>, QosLevel)>, u8> {
        self.subscriptions.get(topic)
    }

    pub fn subscribe(
        &mut self,
        topic: String,
        qos: QosLevel,
        id: u128,
        bridge: Sender<ClientEvent>,
    ) -> SubackReturnCode {
        let leaf = SubscriptionLeaf::new(qos, id, bridge, false, false);

        if let Err(e) = self.subscriptions.insert(topic, leaf) {
            return SubackReturnCode::Failure;
        }

        match qos {
            QosLevel::AtMost => SubackReturnCode::SuccessQosZero,
            QosLevel::AtLeast => SubackReturnCode::SuccessQosOne,
            QosLevel::Exactly => SubackReturnCode::SuccessQosTwo,
        }
    }
    pub fn unsubscribe(&mut self, topic: String, client: u128) -> Result<(), u8> {
        self.subscriptions.delete(topic, client)
    }

    pub async fn disconnect_client(&self, cid: &String) {
        if let Some(client) = self.clients.get(cid) {
            if let Err(e) = client.sender.send(ClientEvent::Disconnect).await {
                error!("{}", e);
            }
        }
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
            for sub in &client.session.subscriptions {}

            self.clients.remove(cid);
        }
    }
}

#[cfg(test)]
mod test {}
