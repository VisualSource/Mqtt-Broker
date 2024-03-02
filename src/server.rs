#[derive(Debug, Default)]
pub struct Info {
    client_count: usize,
    connections_count: usize,
    start_time: usize,
    bytes_recv: usize,
    bytes_send: usize,

    messages_send: usize,
    messages_recv: usize,
}
