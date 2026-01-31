//! MQTT 3.1.1 Protocol Codec
//!
//! This module provides complete packet encoding and decoding for MQTT 3.1.1.
//! It is a pure codec with no I/O - just `&[u8]` ↔ `Packet` transformations.

mod decode;
mod encode;
mod error;
mod packet;
mod types;
mod varint;

pub use decode::decode_packet;
pub use encode::encode_packet;
pub use error::{DecodeError, EncodeError};
pub use packet::*;
pub use types::*;
pub use varint::{decode_variable_int, encode_variable_int};

#[cfg(test)]
mod tests;
