use std::collections::HashMap;

use bytes::Bytes;
use log::{debug, error};
use tokio::sync::mpsc::Sender;

use crate::{
    error::MqttError,
    packets::{
        enums::{QosLevel, SubackReturnCode},
        Packet,
    },
    topic_heir::{SubscriptionLeaf, SubscriptionTree},
};

use self::{
    enums::{ClientEvent, ProtocalVersion},
    session::Session,
};

pub mod broker_info;
pub mod enums;
mod session;

pub struct App {
    sessions: HashMap<String, Session>,
    subscriptions: SubscriptionTree,
}

impl App {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            subscriptions: SubscriptionTree::new(),
        }
    }

    /// Subscribe to the current topic at the given qos
    pub fn subscribe(
        &mut self,
        cid: String,
        topics: Vec<(String, QosLevel)>,
        callback: tokio::sync::oneshot::Sender<Result<Vec<SubackReturnCode>, MqttError>>,
    ) {
        let (id, bridge) = match self.sessions.get(&cid) {
            Some(session) => (session.id, session.bridge.clone()),
            None => {
                if callback.send(Err(MqttError::Unknown)).is_err() {
                    log::error!("Client does not exist");
                }
                return;
            }
        };
        let codes = topics
            .into_iter()
            .map(|(topic, qos)| {
                let leaf = SubscriptionLeaf::new(qos, id, bridge.clone());
                if self.subscriptions.insert(topic, leaf).is_err() {
                    return SubackReturnCode::Failure;
                }
                match qos {
                    QosLevel::AtMost => SubackReturnCode::SuccessQosZero,
                    QosLevel::AtLeast => SubackReturnCode::SuccessQosOne,
                    QosLevel::Exactly => SubackReturnCode::SuccessQosTwo,
                }
            })
            .collect::<Vec<SubackReturnCode>>();

        if callback.send(Ok(codes)).is_err() {
            log::error!("Client does not exist");
        }
    }

    /// Unsubscribe to topic
    pub fn unsubscribe(
        &mut self,
        cid: String,
        topics: Vec<String>,
        callback: tokio::sync::oneshot::Sender<Result<(), MqttError>>,
    ) {
        let id = match self.sessions.get(&cid) {
            Some(session) => session.id,
            None => {
                if callback.send(Err(MqttError::Unknown)).is_err() {
                    log::error!("Client does not exist");
                }
                return;
            }
        };

        topics.into_iter().for_each(|topic| {
            if self.subscriptions.delete(topic, id).is_err() {
                error!("Failed to delete subscription from tree");
            }
        });

        if callback.send(Ok(())).is_err() {
            log::error!("receiver dropped");
        }
    }

    pub async fn connect(
        &mut self,
        client_id: String,
        message_channel: Sender<ClientEvent>,
        _protocol: ProtocalVersion,
        _clean_session: bool,
        callback: tokio::sync::oneshot::Sender<Result<(), MqttError>>,
    ) {
        debug!("New client connecting with id of '{}'", client_id);
        if self.sessions.contains_key(&client_id) {
            if let Some(current_client) = self.sessions.get(&client_id) {
                if let Err(err) = current_client.bridge.send(ClientEvent::Disconnect).await {
                    error!("{}", err);
                }
            }
        }

        if let Some(existing_client) = self.sessions.get_mut(&client_id) {
            existing_client.bridge = message_channel;
        } else {
            self.sessions
                .insert(client_id, Session::new(message_channel));
        }

        if callback.send(Ok(())).is_err() {
            log::error!("Client no longer exists");
        }
    }

    pub fn disconnect(&mut self, cid: String) {
        /*if let Some(client) = self.sessions.get(&cid) {

        }*/
        self.sessions.remove(&cid);
    }

    pub async fn publish(&self, topic: String, payload: Bytes) {
        let subs = match self.subscriptions.get(topic.clone()) {
            Ok(subs) => subs,
            Err(_) => {
                return;
            }
        };

        for (_, bridge, qos) in subs {
            let packet =
                Packet::make_publish(false, qos, false, topic.clone(), None, payload.clone());
            if let Err(e) = bridge.send(ClientEvent::Message(packet)).await {
                log::error!("receiver dropped: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod test {}
