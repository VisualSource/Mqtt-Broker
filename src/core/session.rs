use crate::packets::enums::QosLevel;

pub struct Session {
    pub subscriptions: Vec<(String, QosLevel)>,
}

impl Session {
    pub fn clear(&mut self) {
        self.subscriptions.clear();
    }

    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
        }
    }

    pub fn add_subscription(&mut self, topic: String, qos: QosLevel) {
        self.subscriptions.push((topic, qos));
    }
}
