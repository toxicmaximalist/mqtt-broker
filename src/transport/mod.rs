//! Transport module for network communication.
//!
//! Provides:
//! - TCP connection handling
//! - Frame codec for MQTT packet framing
//! - Connection state management

mod codec;
mod connection;

#[cfg(test)]
mod tests;

pub use codec::{MqttCodec, CodecError, MAX_PACKET_SIZE, DEFAULT_MAX_PACKET_SIZE};
pub use connection::{Connection, ConnectionConfig, ConnectionError};
