//! MQTT packet decoding (bytes → Packet)

use bytes::Bytes;

use crate::codec::error::DecodeError;
use crate::codec::packet::*;
use crate::codec::types::*;
use crate::codec::varint::decode_variable_int;

/// Decode result: either a packet with bytes consumed, or an error
pub type DecodeResult = Result<(Packet, usize), DecodeError>;

/// Decode a single MQTT packet from a byte buffer.
/// Returns the decoded packet and number of bytes consumed.
///
/// If the buffer doesn't contain a complete packet, returns
/// `Err(DecodeError::Incomplete)` - the caller should buffer more data.
pub fn decode_packet(buf: &[u8]) -> DecodeResult {
    if buf.is_empty() {
        return Err(DecodeError::incomplete(1, "fixed header"));
    }

    // Parse fixed header byte
    let header = FixedHeader::from_byte(buf[0]).ok_or(DecodeError::InvalidFlags {
        packet_type: buf[0] >> 4,
        flags: buf[0] & 0x0F,
    })?;

    // Validate packet type
    if header.packet_type == 0 || header.packet_type > 14 {
        return Err(DecodeError::InvalidPacketType(header.packet_type));
    }

    // Parse remaining length
    let (remaining_len, varint_bytes) = decode_variable_int(&buf[1..])?;
    let remaining_len = remaining_len as usize;

    // Calculate total packet size
    let header_size = 1 + varint_bytes;
    let total_size = header_size + remaining_len;

    // Check if we have the complete packet
    if buf.len() < total_size {
        return Err(DecodeError::incomplete(
            total_size - buf.len(),
            "packet payload",
        ));
    }

    // Extract payload slice
    let payload = &buf[header_size..total_size];

    // Decode based on packet type
    let packet = match header.packet_type {
        1 => Packet::Connect(decode_connect(payload)?),
        2 => Packet::ConnAck(decode_connack(payload)?),
        3 => Packet::Publish(decode_publish(payload, header)?),
        4 => Packet::PubAck(decode_puback(payload)?),
        5 => Packet::PubRec(decode_pubrec(payload)?),
        6 => Packet::PubRel(decode_pubrel(payload)?),
        7 => Packet::PubComp(decode_pubcomp(payload)?),
        8 => Packet::Subscribe(decode_subscribe(payload)?),
        9 => Packet::SubAck(decode_suback(payload)?),
        10 => Packet::Unsubscribe(decode_unsubscribe(payload)?),
        11 => Packet::UnsubAck(decode_unsuback(payload)?),
        12 => {
            if remaining_len != 0 {
                return Err(DecodeError::Malformed(
                    "PINGREQ must have zero remaining length".into(),
                ));
            }
            Packet::PingReq
        }
        13 => {
            if remaining_len != 0 {
                return Err(DecodeError::Malformed(
                    "PINGRESP must have zero remaining length".into(),
                ));
            }
            Packet::PingResp
        }
        14 => {
            if remaining_len != 0 {
                return Err(DecodeError::Malformed(
                    "DISCONNECT must have zero remaining length".into(),
                ));
            }
            Packet::Disconnect
        }
        _ => return Err(DecodeError::InvalidPacketType(header.packet_type)),
    };

    Ok((packet, total_size))
}

/// Decode a UTF-8 string with length prefix (2 bytes)
fn decode_string(buf: &[u8], pos: &mut usize) -> Result<String, DecodeError> {
    if buf.len() < *pos + 2 {
        return Err(DecodeError::incomplete(2, "string length"));
    }

    let len = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]) as usize;
    *pos += 2;

    if buf.len() < *pos + len {
        return Err(DecodeError::incomplete(len, "string data"));
    }

    let string_bytes = &buf[*pos..*pos + len];
    *pos += len;

    String::from_utf8(string_bytes.to_vec())
        .map_err(|e| DecodeError::InvalidUtf8(e.to_string()))
}

/// Decode binary data with length prefix (2 bytes)
fn decode_bytes(buf: &[u8], pos: &mut usize) -> Result<Bytes, DecodeError> {
    if buf.len() < *pos + 2 {
        return Err(DecodeError::incomplete(2, "binary length"));
    }

    let len = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]) as usize;
    *pos += 2;

    if buf.len() < *pos + len {
        return Err(DecodeError::incomplete(len, "binary data"));
    }

    let data = Bytes::copy_from_slice(&buf[*pos..*pos + len]);
    *pos += len;

    Ok(data)
}

/// Decode u16 (big-endian)
fn decode_u16(buf: &[u8], pos: &mut usize) -> Result<u16, DecodeError> {
    if buf.len() < *pos + 2 {
        return Err(DecodeError::incomplete(2, "u16"));
    }

    let value = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
    *pos += 2;

    Ok(value)
}

/// Decode CONNECT packet payload
fn decode_connect(buf: &[u8]) -> Result<Connect, DecodeError> {
    let mut pos = 0;

    // Protocol name
    let protocol_name = decode_string(buf, &mut pos)?;
    if protocol_name != "MQTT" && protocol_name != "MQIsdp" {
        return Err(DecodeError::InvalidProtocolName(protocol_name));
    }

    // Protocol level
    if buf.len() < pos + 1 {
        return Err(DecodeError::incomplete(1, "protocol level"));
    }
    let protocol_level = buf[pos];
    pos += 1;

    // We accept level 4 (MQTT 3.1.1) and level 3 (MQTT 3.1)
    // but our implementation targets 3.1.1
    if protocol_level != 4 && protocol_level != 3 {
        return Err(DecodeError::UnsupportedProtocolLevel(protocol_level));
    }

    // Connect flags
    if buf.len() < pos + 1 {
        return Err(DecodeError::incomplete(1, "connect flags"));
    }
    let flags_byte = buf[pos];
    pos += 1;

    let flags =
        ConnectFlags::from_byte(flags_byte).ok_or(DecodeError::InvalidConnectFlags(flags_byte))?;

    if !flags.validate() {
        return Err(DecodeError::InvalidConnectFlags(flags_byte));
    }

    // Keep alive
    let keep_alive = decode_u16(buf, &mut pos)?;

    // Client ID
    let client_id = decode_string(buf, &mut pos)?;

    // Validate client ID (empty allowed only with clean_session=true)
    if client_id.is_empty() && !flags.clean_session {
        return Err(DecodeError::ProtocolViolation(
            "empty client ID requires clean_session=true".into(),
        ));
    }

    // Will topic and message
    let will = if flags.will {
        let will_topic = decode_string(buf, &mut pos)?;
        let will_message = decode_bytes(buf, &mut pos)?;
        Some(Will {
            topic: will_topic,
            message: will_message,
            qos: flags.will_qos,
            retain: flags.will_retain,
        })
    } else {
        None
    };

    // Username
    let username = if flags.username {
        Some(decode_string(buf, &mut pos)?)
    } else {
        None
    };

    // Password
    let password = if flags.password {
        Some(decode_bytes(buf, &mut pos)?)
    } else {
        None
    };

    Ok(Connect {
        protocol_name,
        protocol_level,
        flags,
        keep_alive,
        client_id,
        will,
        username,
        password,
    })
}

/// Decode CONNACK packet payload
fn decode_connack(buf: &[u8]) -> Result<ConnAck, DecodeError> {
    if buf.len() < 2 {
        return Err(DecodeError::incomplete(2, "CONNACK payload"));
    }

    // Byte 1: Connect Acknowledge Flags
    let ack_flags = buf[0];
    if ack_flags & 0xFE != 0 {
        // Reserved bits must be 0
        return Err(DecodeError::Malformed("invalid CONNACK flags".into()));
    }
    let session_present = (ack_flags & 0x01) != 0;

    // Byte 2: Connect Return Code
    let return_code =
        ConnectReturnCode::from_u8(buf[1]).ok_or(DecodeError::InvalidReturnCode(buf[1]))?;

    // Session present must be 0 if return code is not Accepted
    if !return_code.is_accepted() && session_present {
        return Err(DecodeError::ProtocolViolation(
            "session present must be 0 when connection rejected".into(),
        ));
    }

    Ok(ConnAck {
        session_present,
        return_code,
    })
}

/// Decode PUBLISH packet payload
fn decode_publish(buf: &[u8], header: FixedHeader) -> Result<Publish, DecodeError> {
    let mut pos = 0;

    // Topic name
    let topic = decode_string(buf, &mut pos)?;

    // Validate topic name (no wildcards in PUBLISH)
    if topic.is_empty() {
        return Err(DecodeError::InvalidTopicName("topic cannot be empty".into()));
    }
    if topic.contains('+') || topic.contains('#') {
        return Err(DecodeError::InvalidTopicName(
            "wildcards not allowed in PUBLISH topic".into(),
        ));
    }

    // Packet ID (only for QoS > 0)
    let packet_id = if header.qos != QoS::AtMostOnce {
        let id = decode_u16(buf, &mut pos)?;
        if id == 0 {
            return Err(DecodeError::ZeroPacketId);
        }
        Some(id)
    } else {
        None
    };

    // Payload is the remainder
    let payload = Bytes::copy_from_slice(&buf[pos..]);

    Ok(Publish {
        dup: header.dup,
        qos: header.qos,
        retain: header.retain,
        topic,
        packet_id,
        payload,
    })
}

/// Decode PUBACK packet payload
fn decode_puback(buf: &[u8]) -> Result<PubAck, DecodeError> {
    if buf.len() < 2 {
        return Err(DecodeError::incomplete(2, "PUBACK payload"));
    }
    let packet_id = u16::from_be_bytes([buf[0], buf[1]]);
    if packet_id == 0 {
        return Err(DecodeError::ZeroPacketId);
    }
    Ok(PubAck { packet_id })
}

/// Decode PUBREC packet payload
fn decode_pubrec(buf: &[u8]) -> Result<PubRec, DecodeError> {
    if buf.len() < 2 {
        return Err(DecodeError::incomplete(2, "PUBREC payload"));
    }
    let packet_id = u16::from_be_bytes([buf[0], buf[1]]);
    if packet_id == 0 {
        return Err(DecodeError::ZeroPacketId);
    }
    Ok(PubRec { packet_id })
}

/// Decode PUBREL packet payload
fn decode_pubrel(buf: &[u8]) -> Result<PubRel, DecodeError> {
    if buf.len() < 2 {
        return Err(DecodeError::incomplete(2, "PUBREL payload"));
    }
    let packet_id = u16::from_be_bytes([buf[0], buf[1]]);
    if packet_id == 0 {
        return Err(DecodeError::ZeroPacketId);
    }
    Ok(PubRel { packet_id })
}

/// Decode PUBCOMP packet payload
fn decode_pubcomp(buf: &[u8]) -> Result<PubComp, DecodeError> {
    if buf.len() < 2 {
        return Err(DecodeError::incomplete(2, "PUBCOMP payload"));
    }
    let packet_id = u16::from_be_bytes([buf[0], buf[1]]);
    if packet_id == 0 {
        return Err(DecodeError::ZeroPacketId);
    }
    Ok(PubComp { packet_id })
}

/// Decode SUBSCRIBE packet payload
fn decode_subscribe(buf: &[u8]) -> Result<Subscribe, DecodeError> {
    let mut pos = 0;

    // Packet ID
    let packet_id = decode_u16(buf, &mut pos)?;
    if packet_id == 0 {
        return Err(DecodeError::ZeroPacketId);
    }

    // Subscription list
    let mut subscriptions = Vec::new();

    while pos < buf.len() {
        let topic_filter = decode_string(buf, &mut pos)?;

        if buf.len() < pos + 1 {
            return Err(DecodeError::incomplete(1, "subscription QoS"));
        }

        let qos_byte = buf[pos];
        pos += 1;

        // Reserved bits must be 0
        if qos_byte & 0xFC != 0 {
            return Err(DecodeError::Malformed("invalid subscription QoS byte".into()));
        }

        let qos = QoS::from_u8(qos_byte & 0x03).ok_or(DecodeError::InvalidQoS(qos_byte))?;

        subscriptions.push(Subscription { topic_filter, qos });
    }

    if subscriptions.is_empty() {
        return Err(DecodeError::EmptySubscriptions);
    }

    Ok(Subscribe {
        packet_id,
        subscriptions,
    })
}

/// Decode SUBACK packet payload
fn decode_suback(buf: &[u8]) -> Result<SubAck, DecodeError> {
    let mut pos = 0;

    // Packet ID
    let packet_id = decode_u16(buf, &mut pos)?;
    if packet_id == 0 {
        return Err(DecodeError::ZeroPacketId);
    }

    // Return codes
    let mut return_codes = Vec::new();

    while pos < buf.len() {
        let code =
            SubAckCode::from_u8(buf[pos]).ok_or(DecodeError::InvalidReturnCode(buf[pos]))?;
        return_codes.push(code);
        pos += 1;
    }

    if return_codes.is_empty() {
        return Err(DecodeError::Malformed(
            "SUBACK must contain at least one return code".into(),
        ));
    }

    Ok(SubAck {
        packet_id,
        return_codes,
    })
}

/// Decode UNSUBSCRIBE packet payload
fn decode_unsubscribe(buf: &[u8]) -> Result<Unsubscribe, DecodeError> {
    let mut pos = 0;

    // Packet ID
    let packet_id = decode_u16(buf, &mut pos)?;
    if packet_id == 0 {
        return Err(DecodeError::ZeroPacketId);
    }

    // Topic filter list
    let mut topic_filters = Vec::new();

    while pos < buf.len() {
        let topic_filter = decode_string(buf, &mut pos)?;
        topic_filters.push(topic_filter);
    }

    if topic_filters.is_empty() {
        return Err(DecodeError::Malformed(
            "UNSUBSCRIBE must contain at least one topic filter".into(),
        ));
    }

    Ok(Unsubscribe {
        packet_id,
        topic_filters,
    })
}

/// Decode UNSUBACK packet payload
fn decode_unsuback(buf: &[u8]) -> Result<UnsubAck, DecodeError> {
    if buf.len() < 2 {
        return Err(DecodeError::incomplete(2, "UNSUBACK payload"));
    }
    let packet_id = u16::from_be_bytes([buf[0], buf[1]]);
    if packet_id == 0 {
        return Err(DecodeError::ZeroPacketId);
    }
    Ok(UnsubAck { packet_id })
}
