use crate::utils::FromBytes;

use super::{enums::QosLevel, fixed_header::MqttFixedHeader};

pub struct Tuples {
    topic_len: u16,
    topic: String,
    qos: QosLevel,
}
pub struct MqttSubHeader {
    /// SUBSCRIBE|UNSUBSCRIBE Packet fixed header
    header: MqttFixedHeader,

    packet_id: u16,
    tuples_len: u16,
    tuples: Tuples,
}

impl FromBytes for MqttSubHeader {
    type Output = MqttSubHeader;

    fn from_bytes<'a, I>(iter: I, header: MqttFixedHeader) -> Result<Self::Output, ()>
    where
        I: Iterator<Item = &'a u8>,
    {
        todo!()
    }
}
