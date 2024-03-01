/// MQTT v3.1.1 standard, Remaining length field on the fixed header can be at most 4 bytes.
pub const MAX_LEN_BYTES: usize = 4;

/// HEADER length in bytes
pub const MQTT_HEADER_LEN: usize = 2;

pub const MQTT_ACK_LEN: usize = 4;
