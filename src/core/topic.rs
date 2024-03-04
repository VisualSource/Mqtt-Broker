use std::collections::HashMap;

use tokio::sync::mpsc::Sender;

use crate::{packets::enums::QosLevel, server::Event};

use super::pattern::Pattern;

type Subscriber = (Sender<Event>, QosLevel);
type Subscribers = HashMap<String, Subscriber>;

// # Wildcard
// + single level wildcard
// / seperator
pub struct Topics(HashMap<String, Subscribers>);

impl Topics {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get_matches(&self, topic: &String) -> Vec<&Subscriber> {
        let pattern = Pattern::new(topic);

        let data: Vec<&Subscriber> = self
            .0
            .iter()
            .filter_map(|data| {
                if pattern.matches(data.0) {
                    return Some(data.1.values());
                }
                None
            })
            .flatten()
            .collect();

        data
    }

    pub fn unsubscribe(&mut self, cid: &String, topic_name: &String) {
        if !self.0.contains_key(topic_name) {
            return;
        }

        let topic = self.0.get_mut(topic_name).expect("Failed to get table");
        topic.remove(cid);

        if topic.is_empty() {
            self.0.remove(topic_name);
        }
    }

    pub fn subscribe(
        &mut self,
        topic_name: String,
        qos: QosLevel,
        cid: &String,
        tx: Sender<Event>,
    ) -> Result<(), u8> {
        if self.0.contains_key(&topic_name) {
            let topic = self.0.get_mut(&topic_name).ok_or_else(|| 0)?;

            if topic.contains_key(cid) {
                let sub = topic.get_mut(cid).ok_or_else(|| 0)?;
                sub.1 = qos;
                return Ok(());
            }

            topic.insert(cid.clone(), (tx, qos));

            return Ok(());
        }

        let mut sub_table = HashMap::new();

        sub_table.insert(cid.clone(), (tx, qos));

        self.0.insert(topic_name, sub_table);

        Ok(())
    }
}
