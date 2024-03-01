use crate::utils::FromBytes;

use super::fixed_header::MqttFixedHeader;

pub struct MqttSuback {
    header: MqttFixedHeader,
    packet_id: u16,
    rcslen: u16,
    rsc: String,
}

impl FromBytes for MqttSuback {
    type Output = MqttSuback;

    fn from_bytes<'a, I>(iter: I, header: MqttFixedHeader) -> Result<Self::Output, ()>
    where
        I: Iterator<Item = &'a u8>,
    {
        todo!()
    }
}
