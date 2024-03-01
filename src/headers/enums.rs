/// # MQTT Control Packet type
/// Position: byte 1, bits 7-4.
/// Represented as a 4-bit unsigned value.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PacketType {
    /// Client request to connect to Server
    CONNECT = 1,
    /// Connect acknowledgment
    CONNACK = 2,
    /// Publish message
    PUBLISH = 3,
    /// Publish acknowledgment
    PUBACK = 4,
    /// Publish received (assured delivery part 1)
    PUBREC = 5,
    /// Publish release (assured delivery part 2)
    PUBREL = 6,
    /// Publish complete (assured delivery part 3)
    PUBCOMP = 7,
    /// Client subscribe request
    SUBSCRIBE = 8,
    /// Subscribe acknowledgment
    SUBACK = 9,
    /// Unsubscribe request
    UNSUBSCRIBE = 10,
    /// Unsubscribe acknowledgment
    UNSUBACK = 11,
    /// PING request
    PINGREQ = 12,
    /// PING response
    PINGRESP = 13,
    /// Client is disconnecting
    DISCONNECT = 14,
}

impl TryFrom<u8> for PacketType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1..=14 => unsafe { Ok(std::mem::transmute(value)) },
            _ => Err(()),
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QosLevel {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl TryFrom<u8> for QosLevel {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QosLevel::AtMostOnce),
            1 => Ok(QosLevel::AtLeastOnce),
            2 => Ok(QosLevel::ExactlyOnce),
            _ => Err(()),
        }
    }
}
