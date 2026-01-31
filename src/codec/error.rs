//! Codec error types

use thiserror::Error;

/// Errors that can occur during packet decoding
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Not enough data to complete decoding
    #[error("incomplete packet: need {needed} more bytes for {context}")]
    Incomplete {
        needed: usize,
        context: &'static str,
    },

    /// Invalid packet type (0 or >14)
    #[error("invalid packet type: {0}")]
    InvalidPacketType(u8),

    /// Invalid fixed header flags for packet type
    #[error("invalid flags {flags:#04x} for packet type {packet_type}")]
    InvalidFlags { packet_type: u8, flags: u8 },

    /// Malformed variable-length integer
    #[error("malformed variable length integer (exceeds 4 bytes)")]
    MalformedVarint,

    /// Invalid protocol name (not "MQTT" or "MQIsdp")
    #[error("invalid protocol name: {0}")]
    InvalidProtocolName(String),

    /// Unsupported protocol level
    #[error("unsupported protocol level: {0} (expected 4 for MQTT 3.1.1)")]
    UnsupportedProtocolLevel(u8),

    /// Invalid connect flags
    #[error("invalid connect flags: {0:#04x}")]
    InvalidConnectFlags(u8),

    /// Invalid QoS value (> 2)
    #[error("invalid QoS value: {0}")]
    InvalidQoS(u8),

    /// Invalid UTF-8 string
    #[error("invalid UTF-8 string: {0}")]
    InvalidUtf8(String),

    /// Invalid topic name (empty or contains wildcards in PUBLISH)
    #[error("invalid topic name: {0}")]
    InvalidTopicName(String),

    /// Invalid return code
    #[error("invalid return code: {0}")]
    InvalidReturnCode(u8),

    /// Packet too large
    #[error("packet too large: {size} bytes (max {max})")]
    PacketTooLarge { size: usize, max: usize },

    /// Protocol violation
    #[error("protocol violation: {0}")]
    ProtocolViolation(String),

    /// Packet ID is zero (invalid per spec)
    #[error("packet identifier cannot be zero")]
    ZeroPacketId,

    /// Empty subscription list
    #[error("subscription list cannot be empty")]
    EmptySubscriptions,

    /// Generic malformed packet
    #[error("malformed packet: {0}")]
    Malformed(String),
}

impl DecodeError {
    /// Check if this error indicates we need more data
    pub fn is_incomplete(&self) -> bool {
        matches!(self, DecodeError::Incomplete { .. })
    }

    /// Create an incomplete error with context
    pub fn incomplete(needed: usize, context: &'static str) -> Self {
        DecodeError::Incomplete { needed, context }
    }
}

/// Errors that can occur during packet encoding
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// Packet payload too large to encode
    #[error("payload too large: {size} bytes (max remaining length is 268435455)")]
    PayloadTooLarge { size: usize },

    /// Invalid topic name
    #[error("invalid topic name: {0}")]
    InvalidTopicName(String),

    /// Missing required field
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// Invalid packet state
    #[error("invalid packet state: {0}")]
    InvalidState(String),

    /// Buffer write error
    #[error("buffer write error")]
    BufferWrite,
}
