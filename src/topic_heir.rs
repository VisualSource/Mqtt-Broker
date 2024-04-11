use crate::{core::enums::ClientEvent, packets::enums::QosLevel, utils};
use dashmap::DashMap;
use tokio::sync::mpsc::Sender;
// https://github.com/eclipse/mosquitto/blob/master/src/mosquitto_broker_internal.h#L327
// https://github.com/eclipse/mosquitto/blob/master/src/subs.c#L551
// https://github.com/eclipse/mosquitto/blob/master/src/subs.c#L335
//https://github.com/eclipse/mosquitto/blob/master/src/handle_subscribe.c

#[derive(Debug)]
pub struct SubscriptionLeaf {
    qos: QosLevel,
    identifier: u128,
    bridge: Sender<ClientEvent>,
    no_local: bool,
    retain_as_published: bool,
}

impl SubscriptionLeaf {
    pub fn new(
        qos: QosLevel,
        identifier: u128,
        bridge: Sender<ClientEvent>,
        no_local: bool,
        retain_as_published: bool,
    ) -> Self {
        Self {
            qos,
            bridge,
            identifier,
            no_local,
            retain_as_published,
        }
    }
}

#[derive(Debug)]
struct SubHier {
    children: DashMap<String, SubHier>,
    subs: Vec<SubscriptionLeaf>,
    shared: DashMap<String, Vec<SubscriptionLeaf>>,
}
impl SubHier {
    pub fn new() -> Self {
        Self {
            children: DashMap::new(),
            subs: Vec::default(),
            shared: DashMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        mut iter: impl std::iter::Iterator<Item = String>,
        sub: SubscriptionLeaf,
        share: Option<String>,
    ) {
        if let Some(topic) = iter.next() {
            if !self.children.contains_key(&topic) {
                self.children.insert(topic.clone(), Self::new());
            }

            if let Some(mut child) = self.children.get_mut(&topic) {
                child.insert(iter, sub, share);
            }
        } else if let Some(sharename) = share {
            if !self.shared.contains_key(&sharename) {
                self.shared.insert(sharename.clone(), Vec::new());
            }

            if let Some(mut s) = self.shared.get_mut(&sharename) {
                if let Some(idx) = s.iter().position(|e| e.identifier == sub.identifier) {
                    s.remove(idx);
                }
                s.push(sub);
            }
        } else {
            if let Some(idx) = self
                .subs
                .iter()
                .position(|e| e.identifier == sub.identifier)
            {
                self.subs.remove(idx);
            }
            self.subs.push(sub);
        }
    }

    pub fn delete(
        &mut self,
        mut iter: impl std::iter::Iterator<Item = String>,
        identifer: u128,
        share: Option<String>,
    ) -> bool {
        if let Some(topic) = iter.next() {
            if let Some(mut child) = self.children.get_mut(&topic) {
                let can_remove = child.delete(iter, identifer, share);
                // map can deadlock if holding any sort of reference into the map
                // so drop child to prevent deadlock
                drop(child);
                if can_remove {
                    self.children.remove(&topic);
                }
            }
        } else if let Some(sharename) = share {
            if let Some(mut s) = self.shared.get_mut(&sharename) {
                if let Some(idx) = s.iter().position(|e| e.identifier == identifer) {
                    s.remove(idx);
                }
            }
        } else if let Some(idx) = self.subs.iter().position(|e| e.identifier == identifer) {
            self.subs.remove(idx);
        }

        self.children.is_empty() && self.shared.is_empty() && self.subs.is_empty()
    }

    pub fn get(
        &self,
        iter: &mut impl std::iter::Iterator<Item = String>,
        subscribers: &mut Vec<(u128, Sender<ClientEvent>, QosLevel)>,
        share: &Option<String>,
    ) {
        if let Some(topic) = iter.next() {
            if let Some(child) = self.children.get(&topic) {
                child.get(iter, subscribers, share);
            }

            if let Some(child) = self.children.get("+") {
                child.get(iter, subscribers, share);
            }
        } else if let Some(share) = share {
            if let Some(s) = self.shared.get(share) {
                for x in s.iter() {
                    if !subscribers.iter().any(|e| e.0 == x.identifier) {
                        subscribers.push((x.identifier, x.bridge.clone(), x.qos));
                    }
                }
            }
        } else {
            for x in &self.subs {
                if !subscribers.iter().any(|e| e.0 == x.identifier) {
                    subscribers.push((x.identifier, x.bridge.clone(), x.qos));
                }
            }
        }

        if let Some(child) = self.children.get("#") {
            if let Some(share) = share {
                if let Some(s) = child.shared.get(share) {
                    for x in s.iter() {
                        if !subscribers.iter().any(|e| e.0 == x.identifier) {
                            subscribers.push((x.identifier, x.bridge.clone(), x.qos));
                        }
                    }
                }
            } else {
                for x in &child.subs {
                    if !subscribers.iter().any(|e| e.0 == x.identifier) {
                        subscribers.push((x.identifier, x.bridge.clone(), x.qos));
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SubscriptionTree(DashMap<String, SubHier>);

impl SubscriptionTree {
    pub fn new() -> Self {
        Self(DashMap::new())
    }
    pub fn insert(&mut self, filter: String, sub: SubscriptionLeaf) -> Result<(), u8> {
        let (filter_list, sharename) = utils::tokenise_topic(filter)?;

        let mut iter = filter_list.into_iter();

        if let Some(topic) = iter.next() {
            if !self.0.contains_key(&topic) {
                self.0.insert(topic.clone(), SubHier::new());
            }

            let mut child = self.0.get_mut(&topic).unwrap();
            child.insert(iter, sub, sharename);
        }
        Ok(())
    }
    pub fn delete(&mut self, filter: String, sub: u128) -> Result<(), u8> {
        let (filter_list, sharename) = utils::tokenise_topic(filter)?;
        let mut iter = filter_list.into_iter();

        if let Some(topic) = iter.next() {
            if let Some(mut child) = self.0.get_mut(&topic) {
                let can_drop = child.delete(iter, sub, sharename);
                // Map can deadlock if holding any sort of reference into the map
                // so drop child to prevent deadlock
                drop(child);
                if can_drop {
                    self.0.remove(&topic);
                }
            }
        }

        Ok(())
    }
    pub fn get(&self, filter: String) -> Result<Vec<(u128, Sender<ClientEvent>, QosLevel)>, u8> {
        let mut subscribers = Vec::new();
        let (filter_list, sharename) = utils::tokenise_topic(filter)?;

        let mut iter = filter_list.into_iter();

        if let Some(topic) = iter.next() {
            if let Some(child) = self.0.get(&topic) {
                child.get(&mut iter, &mut subscribers, &sharename);
            }
            // single level '+' match
            if let Some(child) = self.0.get("+") {
                child.get(&mut iter, &mut subscribers, &sharename);
            }
        }

        // multi level match
        if let Some(child) = self.0.get("#") {
            if let Some(share) = sharename {
                if let Some(s) = child.shared.get(&share) {
                    for x in s.iter() {
                        if !subscribers.iter().any(|e| e.0 == x.identifier) {
                            subscribers.push((x.identifier, x.bridge.clone(), x.qos));
                        }
                    }
                }
            } else {
                for x in &child.subs {
                    if !subscribers.iter().any(|e| e.0 == x.identifier) {
                        subscribers.push((x.identifier, x.bridge.clone(), x.qos));
                    }
                }
            }
        }

        Ok(subscribers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let (s, _) = tokio::sync::mpsc::channel::<ClientEvent>(1);
        let mut tree = SubscriptionTree::new();

        tree.insert(
            "$share/GroupA/hello/test".to_string(),
            SubscriptionLeaf::new(QosLevel::AtMost, 7, s, false, false),
        )
        .expect("Failed to insert");

        println!("{:#?}", tree);
    }

    #[test]
    fn test_insert_replace() {
        let (s, _) = tokio::sync::mpsc::channel::<ClientEvent>(1);
        let mut tree = SubscriptionTree::new();
        tree.insert(
            "$share/GroupA/hello/test".to_string(),
            SubscriptionLeaf::new(QosLevel::AtMost, 7, s.clone(), false, false),
        )
        .expect("Failed to insert");
        tree.insert(
            "$share/GroupA/hello/test".to_string(),
            SubscriptionLeaf::new(QosLevel::Exactly, 7, s, false, false),
        )
        .expect("Failed to insert");

        println!("{:#?}", tree);
    }

    #[test]
    fn test_delete() {
        let (s, _) = tokio::sync::mpsc::channel::<ClientEvent>(1);
        let mut tree = SubscriptionTree::new();
        tree.insert(
            "/hello/test".to_string(),
            SubscriptionLeaf::new(QosLevel::AtMost, 7, s, false, false),
        )
        .expect("Failed to insert");

        tree.delete("/hello/test".to_string(), 7)
            .expect("Failed to delete");

        println!("{:#?}", tree);
    }

    #[test]
    fn test_get() {
        let (s, _) = tokio::sync::mpsc::channel::<ClientEvent>(1);
        let (sc, _) = tokio::sync::mpsc::channel::<ClientEvent>(1);
        let mut tree = SubscriptionTree::new();
        tree.insert(
            "/hello/test".to_string(),
            SubscriptionLeaf::new(QosLevel::AtMost, 7, s, false, false),
        )
        .expect("Failed to insert");
        tree.insert(
            "/hello/test".to_string(),
            SubscriptionLeaf::new(QosLevel::AtMost, 34, sc.clone(), false, false),
        )
        .expect("Failed to insert");
        tree.insert(
            "$share/GroupA/hello/test".to_string(),
            SubscriptionLeaf::new(QosLevel::AtMost, 34, sc, false, false),
        )
        .expect("Failed to insert");

        let subscribers = tree.get("/hello/test".to_string()).expect("Failed to get");

        println!("{:#?}", subscribers);
        println!("{:#?}", tree);

        let subscribers = tree
            .get("$share/GroupA/hello/test".to_string())
            .expect("Failed to get");
        println!("{:#?}", subscribers)
    }

    #[test]
    fn test_get_single_wild() {
        let (s, _) = tokio::sync::mpsc::channel::<ClientEvent>(0);
        let mut tree = SubscriptionTree::new();
        tree.insert(
            "/+/test".to_string(),
            SubscriptionLeaf::new(QosLevel::AtMost, 7, s, false, false),
        )
        .expect("Failed to insert");

        let subscribers = tree
            .get("/hello/test".to_string())
            .expect("Failed to get subscribers");

        println!("{:#?}", subscribers);
    }

    #[test]
    fn test_get_mutli_wild() {
        let (s, _) = tokio::sync::mpsc::channel::<ClientEvent>(1);
        let mut tree = SubscriptionTree::new();
        tree.insert(
            "#".to_string(),
            SubscriptionLeaf::new(QosLevel::AtMost, 7, s, false, false),
        )
        .expect("Failed to insert");

        let subscribers = tree
            .get("/hello/test".to_string())
            .expect("Failed to get subscribers");

        println!("{:#?}", subscribers);
    }
}
