use std::sync::atomic::{AtomicUsize, Ordering};

/// The total number of bytes received since the broker started.
static BYTES_RECEIVED: AtomicUsize = AtomicUsize::new(0);
/// The total number of bytes sent since the broker started.
static BYTES_SENT: AtomicUsize = AtomicUsize::new(0);
/// The number of currently connected clients
static CLIENTS_CONNECTED: AtomicUsize = AtomicUsize::new(0);
///  The total number of messages of any type received since the broker started.
static MESSAGES_RECEIVED: AtomicUsize = AtomicUsize::new(0);
/// The total number of messages of any type sent since the broker started.
static MESSAGES_SENT: AtomicUsize = AtomicUsize::new(0);
/// The total number of PUBLISH messages received since the broker started.
static MESSAGES_PUBLISH_RECEIVED: AtomicUsize = AtomicUsize::new(0);
/// The total number of PUBLISH messages sent since the broker started.
static MESSAGES_PUBLISH_SENT: AtomicUsize = AtomicUsize::new(0);

pub fn get_stats() -> (usize, usize, usize, usize, usize, usize, usize) {
    (
        BYTES_RECEIVED.load(Ordering::Relaxed),
        BYTES_SENT.load(Ordering::Relaxed),
        CLIENTS_CONNECTED.load(Ordering::Relaxed),
        MESSAGES_RECEIVED.load(Ordering::Relaxed),
        MESSAGES_SENT.load(Ordering::Relaxed),
        MESSAGES_PUBLISH_RECEIVED.load(Ordering::Relaxed),
        MESSAGES_PUBLISH_SENT.load(Ordering::Relaxed),
    )
}

pub fn received_published() {
    MESSAGES_PUBLISH_RECEIVED.fetch_add(1, Ordering::Relaxed);
}

pub fn sent_published() {
    MESSAGES_PUBLISH_SENT.fetch_add(1, Ordering::Relaxed);
}

pub fn received_data(bytes_received: usize) {
    BYTES_RECEIVED.fetch_add(bytes_received, Ordering::Relaxed);
    MESSAGES_RECEIVED.fetch_add(1, Ordering::Relaxed);
}

pub fn sent_data(sent_bytes: usize) {
    BYTES_SENT.fetch_add(sent_bytes, Ordering::Relaxed);
    MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);
}

/// incress connected clients count
pub fn client_inc() {
    CLIENTS_CONNECTED.fetch_add(1, Ordering::Relaxed);
}

/// Descress connected clients count
pub fn client_dec() {
    CLIENTS_CONNECTED.fetch_sub(1, Ordering::Relaxed);
}
