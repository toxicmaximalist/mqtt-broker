//! MQTT primitive types and flags

use bytes::Bytes;

/// Quality of Service levels per MQTT 3.1.1 §4.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum QoS {
    /// At most once delivery (fire and forget)
    #[default]
    AtMostOnce = 0,
    /// At least once delivery (acknowledged)
    AtLeastOnce = 1,
    /// Exactly once delivery (4-way handshake)
    ExactlyOnce = 2,
}

impl QoS {
    /// Create QoS from u8, returning None for invalid values
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(QoS::AtMostOnce),
            1 => Some(QoS::AtLeastOnce),
            2 => Some(QoS::ExactlyOnce),
            _ => None,
        }
    }

    /// Get the maximum of two QoS levels (for subscription downgrade)
    pub fn min(self, other: QoS) -> QoS {
        match (self as u8).min(other as u8) {
            0 => QoS::AtMostOnce,
            1 => QoS::AtLeastOnce,
            _ => QoS::ExactlyOnce,
        }
    }
}

/// CONNECT packet flags (byte 8 of variable header)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConnectFlags {
    pub clean_session: bool,
    pub will: bool,
    pub will_qos: QoS,
    pub will_retain: bool,
    pub username: bool,
    pub password: bool,
}

impl ConnectFlags {
    /// Parse connect flags from a byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        // Bit 0 is reserved and MUST be 0
        if byte & 0x01 != 0 {
            return None;
        }

        let will_qos = QoS::from_u8((byte >> 3) & 0x03)?;

        Some(ConnectFlags {
            clean_session: (byte & 0x02) != 0,
            will: (byte & 0x04) != 0,
            will_qos,
            will_retain: (byte & 0x20) != 0,
            username: (byte & 0x40) != 0,
            password: (byte & 0x80) != 0,
        })
    }

    /// Encode connect flags to a byte
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.clean_session {
            byte |= 0x02;
        }
        if self.will {
            byte |= 0x04;
        }
        byte |= (self.will_qos as u8) << 3;
        if self.will_retain {
            byte |= 0x20;
        }
        if self.username {
            byte |= 0x40;
        }
        if self.password {
            byte |= 0x80;
        }
        byte
    }

    /// Validate flag combinations per MQTT spec
    pub fn validate(&self) -> bool {
        // If will flag is 0, will_qos and will_retain must be 0
        if !self.will && (self.will_qos != QoS::AtMostOnce || self.will_retain) {
            return false;
        }
        // Password flag requires username flag
        if self.password && !self.username {
            return false;
        }
        true
    }
}

/// CONNACK return codes per MQTT 3.1.1 §3.2.2.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectReturnCode {
    Accepted = 0x00,
    UnacceptableProtocolVersion = 0x01,
    IdentifierRejected = 0x02,
    ServerUnavailable = 0x03,
    BadUsernameOrPassword = 0x04,
    NotAuthorized = 0x05,
}

impl ConnectReturnCode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(ConnectReturnCode::Accepted),
            0x01 => Some(ConnectReturnCode::UnacceptableProtocolVersion),
            0x02 => Some(ConnectReturnCode::IdentifierRejected),
            0x03 => Some(ConnectReturnCode::ServerUnavailable),
            0x04 => Some(ConnectReturnCode::BadUsernameOrPassword),
            0x05 => Some(ConnectReturnCode::NotAuthorized),
            _ => None,
        }
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self, ConnectReturnCode::Accepted)
    }
}

/// SUBACK return codes per MQTT 3.1.1 §3.9.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAckCode {
    SuccessQoS0,
    SuccessQoS1,
    SuccessQoS2,
    Failure,
}

impl SubAckCode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(SubAckCode::SuccessQoS0),
            0x01 => Some(SubAckCode::SuccessQoS1),
            0x02 => Some(SubAckCode::SuccessQoS2),
            0x80 => Some(SubAckCode::Failure),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            SubAckCode::SuccessQoS0 => 0x00,
            SubAckCode::SuccessQoS1 => 0x01,
            SubAckCode::SuccessQoS2 => 0x02,
            SubAckCode::Failure => 0x80,
        }
    }

    pub fn from_qos(qos: QoS) -> Self {
        match qos {
            QoS::AtMostOnce => SubAckCode::SuccessQoS0,
            QoS::AtLeastOnce => SubAckCode::SuccessQoS1,
            QoS::ExactlyOnce => SubAckCode::SuccessQoS2,
        }
    }
}

/// Packet identifier (16-bit) used for QoS 1/2 message tracking
pub type PacketId = u16;

/// Last Will and Testament configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Will {
    pub topic: String,
    pub message: Bytes,
    pub qos: QoS,
    pub retain: bool,
}

/// Topic subscription with requested QoS
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscription {
    pub topic_filter: String,
    pub qos: QoS,
}

/// Fixed header first byte breakdown
#[derive(Debug, Clone, Copy)]
pub struct FixedHeader {
    pub packet_type: u8,
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
}

impl FixedHeader {
    /// Parse from first byte of packet
    pub fn from_byte(byte: u8) -> Option<Self> {
        let packet_type = byte >> 4;
        let flags = byte & 0x0F;

        // Validate flags based on packet type per MQTT 3.1.1 §2.2.2
        let (dup, qos, retain) = match packet_type {
            // PUBLISH: flags are variable
            3 => {
                let dup = (flags & 0x08) != 0;
                let qos = QoS::from_u8((flags >> 1) & 0x03)?;
                let retain = (flags & 0x01) != 0;
                (dup, qos, retain)
            }
            // PUBREL, SUBSCRIBE, UNSUBSCRIBE: flags must be 0010
            6 | 8 | 10 => {
                if flags != 0x02 {
                    return None;
                }
                (false, QoS::AtMostOnce, false)
            }
            // All others: flags must be 0000
            _ => {
                if flags != 0x00 {
                    return None;
                }
                (false, QoS::AtMostOnce, false)
            }
        };

        Some(FixedHeader {
            packet_type,
            dup,
            qos,
            retain,
        })
    }

    /// Encode to first byte
    pub fn to_byte(&self) -> u8 {
        let mut byte = self.packet_type << 4;
        if self.dup {
            byte |= 0x08;
        }
        byte |= (self.qos as u8) << 1;
        if self.retain {
            byte |= 0x01;
        }
        byte
    }
}

/// Protocol level constants
pub const PROTOCOL_NAME: &str = "MQTT";
pub const PROTOCOL_LEVEL_3_1_1: u8 = 0x04;

/// Maximum remaining length (256MB)
pub const MAX_REMAINING_LENGTH: usize = 268_435_455;

/// Maximum packet size we'll accept (configurable, default 1MB)
pub const DEFAULT_MAX_PACKET_SIZE: usize = 1_048_576;
