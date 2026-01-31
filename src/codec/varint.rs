//! Variable-length integer encoding per MQTT 3.1.1 §2.2.3

use crate::codec::error::DecodeError;

/// Maximum bytes for variable length encoding (4 bytes = 268,435,455 max)
pub const MAX_VARINT_BYTES: usize = 4;

/// Decode a variable-length integer from bytes.
/// Returns (value, bytes_consumed) or error.
///
/// Per MQTT 3.1.1 §2.2.3:
/// - Each byte uses 7 bits for value, 1 bit (MSB) as continuation flag
/// - Max 4 bytes (max value 268,435,455)
pub fn decode_variable_int(bytes: &[u8]) -> Result<(u32, usize), DecodeError> {
    let mut multiplier: u32 = 1;
    let mut value: u32 = 0;
    let mut pos = 0;

    loop {
        if pos >= bytes.len() {
            return Err(DecodeError::Incomplete {
                needed: 1,
                context: "variable integer",
            });
        }

        let byte = bytes[pos];
        value += (byte & 0x7F) as u32 * multiplier;

        if multiplier > 128 * 128 * 128 {
            return Err(DecodeError::MalformedVarint);
        }

        pos += 1;

        if (byte & 0x80) == 0 {
            break;
        }

        multiplier *= 128;
    }

    Ok((value, pos))
}

/// Encode a value as variable-length integer.
/// Returns the encoded bytes (1-4 bytes).
///
/// Per MQTT 3.1.1 §2.2.3
pub fn encode_variable_int(mut value: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4);

    loop {
        let mut byte = (value % 128) as u8;
        value /= 128;

        if value > 0 {
            byte |= 0x80; // Set continuation bit
        }

        bytes.push(byte);

        if value == 0 {
            break;
        }
    }

    bytes
}

/// Calculate the number of bytes needed to encode a value
pub fn varint_len(value: u32) -> usize {
    if value < 128 {
        1
    } else if value < 16_384 {
        2
    } else if value < 2_097_152 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let test_values = [
            0,
            1,
            127,
            128,
            16383,
            16384,
            2097151,
            2097152,
            268435455,
        ];

        for &value in &test_values {
            let encoded = encode_variable_int(value);
            let (decoded, len) = decode_variable_int(&encoded).unwrap();
            assert_eq!(decoded, value, "Roundtrip failed for {}", value);
            assert_eq!(len, encoded.len());
        }
    }

    #[test]
    fn test_specific_encodings() {
        // Per MQTT spec examples
        assert_eq!(encode_variable_int(0), vec![0x00]);
        assert_eq!(encode_variable_int(127), vec![0x7F]);
        assert_eq!(encode_variable_int(128), vec![0x80, 0x01]);
        assert_eq!(encode_variable_int(16383), vec![0xFF, 0x7F]);
        assert_eq!(encode_variable_int(16384), vec![0x80, 0x80, 0x01]);
        assert_eq!(encode_variable_int(2097151), vec![0xFF, 0xFF, 0x7F]);
        assert_eq!(encode_variable_int(2097152), vec![0x80, 0x80, 0x80, 0x01]);
        assert_eq!(
            encode_variable_int(268435455),
            vec![0xFF, 0xFF, 0xFF, 0x7F]
        );
    }

    #[test]
    fn test_varint_len() {
        assert_eq!(varint_len(0), 1);
        assert_eq!(varint_len(127), 1);
        assert_eq!(varint_len(128), 2);
        assert_eq!(varint_len(16383), 2);
        assert_eq!(varint_len(16384), 3);
        assert_eq!(varint_len(2097151), 3);
        assert_eq!(varint_len(2097152), 4);
        assert_eq!(varint_len(268435455), 4);
    }

    #[test]
    fn test_incomplete_varint() {
        // Continuation bit set but no more bytes
        let result = decode_variable_int(&[0x80]);
        assert!(matches!(result, Err(DecodeError::Incomplete { .. })));
    }

    #[test]
    fn test_empty_buffer() {
        let result = decode_variable_int(&[]);
        assert!(matches!(result, Err(DecodeError::Incomplete { .. })));
    }
}
