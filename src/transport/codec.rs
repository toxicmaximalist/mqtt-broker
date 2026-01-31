//! MQTT frame codec for tokio.
//!
//! Implements encoding and decoding of MQTT packets over a byte stream.

use bytes::{BufMut, BytesMut};

use crate::codec::{Packet, DecodeError, EncodeError, decode_packet, encode_packet};

/// Maximum MQTT packet size (256 MB per spec).
pub const MAX_PACKET_SIZE: usize = 256 * 1024 * 1024;

/// Default maximum packet size (1 MB).
pub const DEFAULT_MAX_PACKET_SIZE: usize = 1024 * 1024;

/// MQTT codec for framing packets over a byte stream.
///
/// This codec handles the variable-length encoding of MQTT packets,
/// accumulating bytes until a complete packet is available.
#[derive(Debug)]
pub struct MqttCodec {
    /// Maximum allowed packet size.
    max_packet_size: usize,
    /// State for decoding remaining length.
    decode_state: DecodeState,
}

#[derive(Debug, Default)]
struct DecodeState {
    /// Accumulated remaining length bytes.
    remaining_length_bytes: Vec<u8>,
    /// Decoded remaining length (once known).
    remaining_length: Option<usize>,
    /// Fixed header byte (once read).
    fixed_header: Option<u8>,
}

impl DecodeState {
    fn reset(&mut self) {
        self.remaining_length_bytes.clear();
        self.remaining_length = None;
        self.fixed_header = None;
    }
}

impl MqttCodec {
    /// Create a new MQTT codec with default max packet size.
    pub fn new() -> Self {
        Self {
            max_packet_size: DEFAULT_MAX_PACKET_SIZE,
            decode_state: DecodeState::default(),
        }
    }

    /// Create a new MQTT codec with custom max packet size.
    pub fn with_max_packet_size(max_packet_size: usize) -> Self {
        Self {
            max_packet_size: max_packet_size.min(MAX_PACKET_SIZE),
            decode_state: DecodeState::default(),
        }
    }

    /// Decode a packet from the buffer.
    ///
    /// Returns `Ok(Some(packet))` if a complete packet was decoded,
    /// `Ok(None)` if more data is needed, or an error.
    pub fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Packet>, CodecError> {
        // Need at least 2 bytes for minimum packet (fixed header + 0 remaining length)
        if buf.is_empty() {
            return Ok(None);
        }

        // Read fixed header if not already done
        if self.decode_state.fixed_header.is_none() {
            self.decode_state.fixed_header = Some(buf[0]);
        }

        // Try to decode remaining length
        if self.decode_state.remaining_length.is_none() {
            let start_pos = 1 + self.decode_state.remaining_length_bytes.len();
            
            // Read remaining length bytes
            while start_pos + self.decode_state.remaining_length_bytes.len() - 1 < buf.len() {
                let idx = 1 + self.decode_state.remaining_length_bytes.len();
                if idx >= buf.len() {
                    return Ok(None); // Need more data
                }

                let byte = buf[idx];
                self.decode_state.remaining_length_bytes.push(byte);

                // Check if this is the last byte (MSB not set)
                if byte & 0x80 == 0 {
                    // Decode the remaining length
                    let mut multiplier = 1u32;
                    let mut value = 0u32;
                    
                    for &b in &self.decode_state.remaining_length_bytes {
                        value += (b as u32 & 0x7F) * multiplier;
                        multiplier *= 128;
                        
                        if multiplier > 128 * 128 * 128 {
                            // Already processed 4 bytes
                            break;
                        }
                    }

                    let remaining_length = value as usize;
                    
                    // Check size limit
                    if remaining_length > self.max_packet_size {
                        self.decode_state.reset();
                        return Err(CodecError::PacketTooLarge {
                            size: remaining_length,
                            max: self.max_packet_size,
                        });
                    }

                    self.decode_state.remaining_length = Some(remaining_length);
                    break;
                }

                // Too many remaining length bytes
                if self.decode_state.remaining_length_bytes.len() > 4 {
                    self.decode_state.reset();
                    return Err(CodecError::MalformedRemainingLength);
                }
            }
        }

        // If we don't have remaining length yet, need more data
        let Some(remaining_length) = self.decode_state.remaining_length else {
            return Ok(None);
        };

        // Calculate total packet size
        let header_size = 1 + self.decode_state.remaining_length_bytes.len();
        let total_size = header_size + remaining_length;

        // Check if we have the complete packet
        if buf.len() < total_size {
            return Ok(None);
        }

        // Extract the packet bytes
        let packet_bytes = buf.split_to(total_size);

        // Reset decode state for next packet
        self.decode_state.reset();

        // Decode the packet - decode_packet returns (Packet, bytes_consumed)
        let (packet, _) = decode_packet(&mut packet_bytes.as_ref())?;
        Ok(Some(packet))
    }

    /// Encode a packet into the buffer.
    pub fn encode(&mut self, packet: &Packet, buf: &mut BytesMut) -> Result<(), CodecError> {
        let encoded = encode_packet(packet)?;
        
        // Check size limit
        if encoded.len() > self.max_packet_size {
            return Err(CodecError::PacketTooLarge {
                size: encoded.len(),
                max: self.max_packet_size,
            });
        }

        buf.reserve(encoded.len());
        buf.put_slice(&encoded);
        Ok(())
    }

    /// Get the maximum packet size.
    pub fn max_packet_size(&self) -> usize {
        self.max_packet_size
    }
}

impl Default for MqttCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during codec operations.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("Packet too large: {size} bytes (max: {max})")]
    PacketTooLarge { size: usize, max: usize },

    #[error("Malformed remaining length encoding")]
    MalformedRemainingLength,

    #[error("Decode error: {0}")]
    Decode(#[from] DecodeError),

    #[error("Encode error: {0}")]
    Encode(#[from] EncodeError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
