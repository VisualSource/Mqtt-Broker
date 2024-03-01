use crate::utils::{unpack_u16, FromBytes};

use super::fixed_header::MqttFixedHeader;

pub struct MqttAck {
    header: MqttFixedHeader,
    packet_id: u16,
}

impl FromBytes for MqttAck {
    type Output = MqttAck;

    fn from_bytes<'a, I>(mut iter: I, header: MqttFixedHeader) -> Result<Self::Output, ()>
    where
        I: Iterator<Item = &'a u8>,
    {
        let packet_id = unpack_u16(&mut iter)?;

        Ok(MqttAck { header, packet_id })
    }
}
