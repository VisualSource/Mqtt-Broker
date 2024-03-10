use std::sync::Arc;

use bytes::Bytes;
use log::debug;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use crate::{
    core::{broker_info, queue::FifoQueue},
    error::MqttError,
};

pub async fn handle_write_stream<Writer>(
    cancellation: CancellationToken,
    mut writer: Writer,
    queue: Arc<FifoQueue<Bytes>>,
    task_id: usize,
) -> Result<(), MqttError>
where
    Writer: AsyncWrite + Unpin,
{
    debug!("(Task {}) Client: Staring Write handler", task_id);
    loop {
        let empty = queue.is_empty()?;
        if empty && cancellation.is_cancelled() {
            break;
        }
        if !empty {
            if let Some(message) = queue.pop()? {
                let len = writer.write(&message).await?;
                debug!("(Task {}) Wrote {} bytes", task_id, len);
                broker_info::received_data(len);
            }
        }
    }

    cancellation.cancel();
    writer.shutdown().await?;

    debug!("(Task {}) Client: Exiting write handler", task_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;
    use std::{sync::Arc, time::Duration};
    use tokio_test::assert_ok;

    use bytes::Bytes;
    use tokio_util::sync::CancellationToken;

    use crate::core::queue::FifoQueue;

    fn init_logger() {
        let _ = env_logger::builder()
            .filter(None, log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_write_stream_error() {
        init_logger();
        let token = CancellationToken::new();
        let queue = Arc::new(FifoQueue::<Bytes>::new());

        queue
            .push(Bytes::from_static(b"hello"))
            .expect("Failed to add message to queue");

        let writer = tokio_test::io::Builder::new()
            .write_error(std::io::Error::from(std::io::ErrorKind::Other))
            .build();

        let handle =
            tokio::spawn(async move { handle_write_stream(token, writer, queue, 0).await });

        tokio::time::sleep(Duration::from_secs(5)).await;

        let result = handle.await.expect("Failed to join");

        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_write_stream_handler() {
        init_logger();

        let token = CancellationToken::new();
        let queue = Arc::new(FifoQueue::<Bytes>::new());

        queue
            .push(Bytes::from_static(b"hello"))
            .expect("Failed to add message to queue");

        let writer = tokio_test::io::Builder::new().write(b"hello").build();

        let token_ref = token.clone();
        let handle =
            tokio::spawn(async move { handle_write_stream(token_ref, writer, queue, 0).await });

        info!("Sleep for 5 sec");
        tokio::time::sleep(Duration::from_secs(5)).await;
        info!("Cancel");
        token.cancel();
        info!("Join");
        let result = handle.await;

        assert!(result.is_ok());
        assert_ok!(result.expect("Failed to unwrap"));
    }
}
