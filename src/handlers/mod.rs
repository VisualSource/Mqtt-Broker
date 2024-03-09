use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use log::{debug, info};
use tokio::{
    net::TcpStream,
    select,
    sync::mpsc::{channel, Sender},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{
    core::{
        broker_info,
        enums::{ClientEvent, Command},
        queue::FifoQueue,
    },
    error::MqttError,
};

mod message_handler;
mod read_handler;
mod write_handler;

pub async fn client_handler(
    stream: TcpStream,
    message_bridge: Sender<Command>,
    cancellation_external: CancellationToken,
) -> Result<(), MqttError> {
    broker_info::client_inc();
    info!(
        "Client starting from: {}",
        stream.peer_addr().map_err(MqttError::Io)?
    );

    let token = CancellationToken::new();
    let (reader, writer) = stream.into_split();
    let reply_queue = Arc::new(FifoQueue::<Bytes>::new());
    let (tx, rx) = channel::<ClientEvent>(100);

    let rh_t = token.clone();
    let rh_rq = reply_queue.clone();
    let read_handle = tokio::spawn(async move {
        read_handler::handle_read_stream(message_bridge, rh_t, rh_rq, tx, reader).await
    });

    let wh_t = token.clone();
    let wh_rq = reply_queue.clone();
    let write_handle =
        tokio::spawn(async move { write_handler::handle_write_stream(wh_t, writer, wh_rq).await });

    let mh_t = token.clone();
    let mh_rq = reply_queue.clone();
    let message_handle =
        tokio::spawn(async move { message_handler::message_handler(mh_t, mh_rq, rx).await });

    select! {
        _ =  cancellation_external.cancelled() => {
            debug!("Client: External cancelled");
            token.cancel();
        }
        _ = token.cancelled() => {
            debug!("Client: Internal canclled");
        }
    }

    broker_info::client_dec();

    message_handle.await??;
    write_handle.await??;
    read_handle.await??;

    info!("Client: disconnect");

    Ok(())
}
