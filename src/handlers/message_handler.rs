use std::sync::Arc;

use bytes::Bytes;
use log::debug;
use tokio::{select, sync::mpsc::Receiver};
use tokio_util::sync::CancellationToken;

use crate::{
    core::{enums::ClientEvent, queue::FifoQueue},
    error::MqttError,
};

pub async fn message_handler(
    token: CancellationToken,
    queue: Arc<FifoQueue<Bytes>>,
    mut rx: Receiver<ClientEvent>,
    task_id: usize,
) -> Result<(), MqttError> {
    debug!("(Task {}) Client: Start message handler", task_id);
    loop {
        select! {
            () = token.cancelled() => break,
            data = rx.recv() => {
                if let Some(message) = data {
                    match message {
                        ClientEvent::Message(payload) => {
                            queue.push(payload)?;
                        }
                        ClientEvent::Disconnect => {
                            break;
                        }
                    }
                }
            }
        }
    }

    token.cancel();

    debug!("(Task {}) Exiting Message Handler", task_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio_test::assert_ok;

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_handler() {
        let queue = Arc::new(FifoQueue::new());
        let token = CancellationToken::new();

        let (_, rx) = tokio::sync::mpsc::channel(1);

        let t = token.clone();
        let a = tokio::spawn(async move { message_handler(t, queue, rx, 0).await });

        tokio::time::sleep(Duration::from_secs(10)).await;

        token.cancel();

        let reuslt = a.await.expect("Failed to join");

        assert_ok!(reuslt)
    }
}
