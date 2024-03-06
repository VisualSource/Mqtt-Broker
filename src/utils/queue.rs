use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
};

use crate::error::MqttError;
// https://github.com/rawnly/queue-rs/blob/main/examples/queue.rs
pub struct FifoQueue<T> {
    data: Mutex<VecDeque<T>>,
    cv: Condvar,
}

impl<T> FifoQueue<T> {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
        }
    }

    pub fn push(&self, value: T) -> Result<(), MqttError> {
        let mut data = self.data.lock().map_err(|_| MqttError::QueuePoisonError)?;
        data.push_back(value);

        self.cv.notify_one();

        Ok(())
    }

    pub fn pop(&self) -> Result<Option<T>, MqttError> {
        let mut data = self.data.lock().map_err(|_| MqttError::QueuePoisonError)?;

        while data.is_empty() {
            data = self
                .cv
                .wait(data)
                .map_err(|_| MqttError::QueuePoisonError)?;
        }

        Ok(data.pop_front())
    }

    pub fn len(&self) -> Result<usize, MqttError> {
        let data = self.data.lock().map_err(|_| MqttError::QueuePoisonError)?;

        Ok(data.len())
    }

    pub fn is_empty(&self) -> Result<bool, MqttError> {
        let data = self.data.lock().map_err(|_| MqttError::QueuePoisonError)?;
        Ok(data.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::FifoQueue;
    use std::{sync::Arc, thread};

    #[test]
    fn test_queue_thread_safety() {
        let queue = Arc::new(FifoQueue::<i32>::new());

        let q1 = queue.clone();
        let t1 = thread::spawn(move || {
            if let Err(err) = q1.push(1) {
                panic!("{}", err);
            }
            if let Err(err) = q1.push(2) {
                panic!("{}", err);
            }
        });

        let q2 = queue.clone();
        let t2 = thread::spawn(move || {
            if let Err(err) = q2.push(3) {
                panic!("{}", err);
            }
            if let Err(err) = q2.push(4) {
                panic!("{}", err);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert_eq!(queue.len().unwrap(), 4);
    }
}
