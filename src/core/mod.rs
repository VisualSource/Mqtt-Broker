use std::collections::HashMap;

use tokio::sync::mpsc::Sender;

use crate::{packets::enums::QosLevel, server::Event};

use self::{client::Client, topic::Topics};

mod client;
pub mod info;
mod pattern;
mod session;
mod topic;

pub struct App {
    pub clients: HashMap<String, Client>,
    pub topics: Topics,
}

impl App {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            topics: Topics::new(),
        }
    }

    pub fn add_client(&mut self, cid: String, tx: Sender<Event>) {
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
        tx: Sender<Event>,
    ) -> Result<(), u8> {
        self.topics.subscribe(topic_name, qos, cid, tx)?;

        Ok(())
    }

    pub fn remove_subscriber_from_topic(&mut self, topic_name: &String, cid: &String) {
        self.topics.unsubscribe(cid, topic_name);
    }
}

#[cfg(test)]
mod test {}
