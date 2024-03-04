use std::mem::size_of;

use crate::{
    error::MqttError,
    packets::{
        enums::QosLevel,
        traits::{FromBytes, ToBytes},
        utils::{unpack_string, unpack_u16},
    },
};

#[derive(Debug)]
pub struct Tuple {
    pub topic: String,
    pub qos: QosLevel,
}

impl Tuple {
    pub fn new(topic: String, qos: QosLevel) -> Self {
        Self { topic, qos }
    }
}

#[derive(Debug, Default)]
pub struct SubscribeHeader {
    pub packet_id: u16,
    pub tuples: Vec<Tuple>,
}

impl SubscribeHeader {
    pub fn new(packet_id: u16, tuples: Vec<Tuple>) -> Self {
        Self { packet_id, tuples }
    }
}

impl FromBytes for SubscribeHeader {
    type Output = SubscribeHeader;

    fn from_bytes<'a, I>(
        iter: &mut I,
        header: Option<&super::fixed_header::FixedHeader>,
    ) -> Result<Self::Output, crate::error::MqttError>
    where
        I: Iterator<Item = &'a u8>,
    {
        let h = header.ok_or_else(|| MqttError::MissingFixedHeader)?;

        let mut len = h.get_remaing_len();

        let mut sub = SubscribeHeader::default();

        sub.packet_id = unpack_u16(iter)?;
        len -= size_of::<u16>();

        /*
         * Read in a loop all remaining bytes specified by len of the Fixed Header.
         * From now on the payload consists of 3-tuples formed by:
         *  - topic filter (string)
         *  - qos
         */

        while len > 0 {
            len -= size_of::<u16>();

            let topic = unpack_string(iter)?;
            len -= topic.len();

            let qos = QosLevel::try_from(*iter.next().ok_or_else(|| MqttError::MissingByte)?)?;

            len -= size_of::<u8>();

            sub.tuples.push(Tuple { topic, qos });
        }

        Ok(sub)
    }
}

impl ToBytes for SubscribeHeader {
    fn to_bytes(&self) -> Result<Vec<u8>, MqttError> {
        let mut data = vec![self.packet_id.to_be_bytes().to_vec()];

        for x in &self.tuples {
            data.push(
                vec![
                    (x.topic.len() as u16).to_be_bytes().to_vec(),
                    x.topic.as_bytes().to_vec(),
                    vec![x.qos as u8],
                ]
                .concat(),
            );
        }

        Ok(data.concat())
    }
}
