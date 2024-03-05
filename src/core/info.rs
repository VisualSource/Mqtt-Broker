use std::sync::atomic::AtomicUsize;

/// The total number of bytes received since the broker started.
pub static BYTES_RECEIVED: AtomicUsize = AtomicUsize::new(0);
/// The total number of bytes sent since the broker started.
pub static BYTES_SENT: AtomicUsize = AtomicUsize::new(0);
/// The number of currently connected clients
pub static CLIENTS_CONNECTED: AtomicUsize = AtomicUsize::new(0);
///  The total number of messages of any type received since the broker started.
pub static MESSAGES_RECEIVED: AtomicUsize = AtomicUsize::new(0);
/// The total number of messages of any type sent since the broker started.
pub static MESSAGES_SENT: AtomicUsize = AtomicUsize::new(0);
/// The total number of PUBLISH messages received since the broker started.
pub static MESSAGES_PUBLISH_RECEIVED: AtomicUsize = AtomicUsize::new(0);
/// The total number of PUBLISH messages sent since the broker started.
pub static MESSAGES_PUBLISH_SENT: AtomicUsize = AtomicUsize::new(0);
