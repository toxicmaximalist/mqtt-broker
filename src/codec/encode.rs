//! MQTT packet encoding (Packet → bytes)

use bytes::{BufMut, BytesMut};

use crate::codec::error::EncodeError;
use crate::codec::packet::*;
use crate::codec::types::*;
use crate::codec::varint::{encode_variable_int, varint_len};

/// Encode result
pub type EncodeResult = Result<BytesMut, EncodeError>;

/// Encode a packet into a byte buffer
pub fn encode_packet(packet: &Packet) -> EncodeResult {
    match packet {
        Packet::Connect(p) => encode_connect(p),
        Packet::ConnAck(p) => encode_connack(p),
        Packet::Publish(p) => encode_publish(p),
        Packet::PubAck(p) => encode_puback(p),
        Packet::PubRec(p) => encode_pubrec(p),
        Packet::PubRel(p) => encode_pubrel(p),
        Packet::PubComp(p) => encode_pubcomp(p),
        Packet::Subscribe(p) => encode_subscribe(p),
        Packet::SubAck(p) => encode_suback(p),
        Packet::Unsubscribe(p) => encode_unsubscribe(p),
        Packet::UnsubAck(p) => encode_unsuback(p),
        Packet::PingReq => encode_pingreq(),
        Packet::PingResp => encode_pingresp(),
        Packet::Disconnect => encode_disconnect(),
    }
}

/// Encode a length-prefixed UTF-8 string
fn encode_string(buf: &mut BytesMut, s: &str) {
    buf.put_u16(s.len() as u16);
    buf.put_slice(s.as_bytes());
}

/// Encode length-prefixed binary data
fn encode_bytes(buf: &mut BytesMut, data: &[u8]) {
    buf.put_u16(data.len() as u16);
    buf.put_slice(data);
}

/// Calculate encoded string size (2 + len)
fn string_size(s: &str) -> usize {
    2 + s.len()
}

/// Calculate encoded bytes size (2 + len)
fn bytes_size(data: &[u8]) -> usize {
    2 + data.len()
}

/// Write fixed header and remaining length
fn write_header(buf: &mut BytesMut, header_byte: u8, remaining_len: usize) -> Result<(), EncodeError> {
    if remaining_len > MAX_REMAINING_LENGTH {
        return Err(EncodeError::PayloadTooLarge { size: remaining_len });
    }
    buf.put_u8(header_byte);
    buf.put_slice(&encode_variable_int(remaining_len as u32));
    Ok(())
}

/// Encode CONNECT packet
fn encode_connect(packet: &Connect) -> EncodeResult {
    // Calculate remaining length
    let mut remaining_len = 0;

    // Variable header
    remaining_len += string_size(&packet.protocol_name); // Protocol name
    remaining_len += 1; // Protocol level
    remaining_len += 1; // Connect flags
    remaining_len += 2; // Keep alive

    // Payload
    remaining_len += string_size(&packet.client_id);

    if let Some(will) = &packet.will {
        remaining_len += string_size(&will.topic);
        remaining_len += bytes_size(&will.message);
    }

    if let Some(username) = &packet.username {
        remaining_len += string_size(username);
    }

    if let Some(password) = &packet.password {
        remaining_len += bytes_size(password);
    }

    // Allocate buffer
    let total_size = 1 + varint_len(remaining_len as u32) + remaining_len;
    let mut buf = BytesMut::with_capacity(total_size);

    // Fixed header: CONNECT = 0x10
    write_header(&mut buf, 0x10, remaining_len)?;

    // Variable header
    encode_string(&mut buf, &packet.protocol_name);
    buf.put_u8(packet.protocol_level);
    buf.put_u8(packet.flags.to_byte());
    buf.put_u16(packet.keep_alive);

    // Payload
    encode_string(&mut buf, &packet.client_id);

    if let Some(will) = &packet.will {
        encode_string(&mut buf, &will.topic);
        encode_bytes(&mut buf, &will.message);
    }

    if let Some(username) = &packet.username {
        encode_string(&mut buf, username);
    }

    if let Some(password) = &packet.password {
        encode_bytes(&mut buf, password);
    }

    Ok(buf)
}

/// Encode CONNACK packet
fn encode_connack(packet: &ConnAck) -> EncodeResult {
    let mut buf = BytesMut::with_capacity(4);

    // Fixed header: CONNACK = 0x20, remaining length = 2
    buf.put_u8(0x20);
    buf.put_u8(0x02);

    // Variable header
    buf.put_u8(if packet.session_present { 0x01 } else { 0x00 });
    buf.put_u8(packet.return_code as u8);

    Ok(buf)
}

/// Encode PUBLISH packet
fn encode_publish(packet: &Publish) -> EncodeResult {
    // Calculate remaining length
    let mut remaining_len = string_size(&packet.topic);

    if packet.qos != QoS::AtMostOnce {
        remaining_len += 2; // Packet ID
    }

    remaining_len += packet.payload.len();

    // Allocate buffer
    let total_size = 1 + varint_len(remaining_len as u32) + remaining_len;
    let mut buf = BytesMut::with_capacity(total_size);

    // Fixed header
    let header = FixedHeader {
        packet_type: 3,
        dup: packet.dup,
        qos: packet.qos,
        retain: packet.retain,
    };
    write_header(&mut buf, header.to_byte(), remaining_len)?;

    // Variable header
    encode_string(&mut buf, &packet.topic);

    if packet.qos != QoS::AtMostOnce {
        if let Some(packet_id) = packet.packet_id {
            buf.put_u16(packet_id);
        } else {
            return Err(EncodeError::MissingField("packet_id for QoS > 0"));
        }
    }

    // Payload
    buf.put_slice(&packet.payload);

    Ok(buf)
}

/// Encode PUBACK packet
fn encode_puback(packet: &PubAck) -> EncodeResult {
    let mut buf = BytesMut::with_capacity(4);

    // Fixed header: PUBACK = 0x40, remaining length = 2
    buf.put_u8(0x40);
    buf.put_u8(0x02);
    buf.put_u16(packet.packet_id);

    Ok(buf)
}

/// Encode PUBREC packet
fn encode_pubrec(packet: &PubRec) -> EncodeResult {
    let mut buf = BytesMut::with_capacity(4);

    // Fixed header: PUBREC = 0x50, remaining length = 2
    buf.put_u8(0x50);
    buf.put_u8(0x02);
    buf.put_u16(packet.packet_id);

    Ok(buf)
}

/// Encode PUBREL packet
fn encode_pubrel(packet: &PubRel) -> EncodeResult {
    let mut buf = BytesMut::with_capacity(4);

    // Fixed header: PUBREL = 0x62 (flags = 0010), remaining length = 2
    buf.put_u8(0x62);
    buf.put_u8(0x02);
    buf.put_u16(packet.packet_id);

    Ok(buf)
}

/// Encode PUBCOMP packet
fn encode_pubcomp(packet: &PubComp) -> EncodeResult {
    let mut buf = BytesMut::with_capacity(4);

    // Fixed header: PUBCOMP = 0x70, remaining length = 2
    buf.put_u8(0x70);
    buf.put_u8(0x02);
    buf.put_u16(packet.packet_id);

    Ok(buf)
}

/// Encode SUBSCRIBE packet
fn encode_subscribe(packet: &Subscribe) -> EncodeResult {
    if packet.subscriptions.is_empty() {
        return Err(EncodeError::InvalidState(
            "SUBSCRIBE must have at least one subscription".into(),
        ));
    }

    // Calculate remaining length
    let mut remaining_len = 2; // Packet ID

    for sub in &packet.subscriptions {
        remaining_len += string_size(&sub.topic_filter);
        remaining_len += 1; // QoS byte
    }

    // Allocate buffer
    let total_size = 1 + varint_len(remaining_len as u32) + remaining_len;
    let mut buf = BytesMut::with_capacity(total_size);

    // Fixed header: SUBSCRIBE = 0x82 (flags = 0010)
    write_header(&mut buf, 0x82, remaining_len)?;

    // Variable header
    buf.put_u16(packet.packet_id);

    // Payload
    for sub in &packet.subscriptions {
        encode_string(&mut buf, &sub.topic_filter);
        buf.put_u8(sub.qos as u8);
    }

    Ok(buf)
}

/// Encode SUBACK packet
fn encode_suback(packet: &SubAck) -> EncodeResult {
    if packet.return_codes.is_empty() {
        return Err(EncodeError::InvalidState(
            "SUBACK must have at least one return code".into(),
        ));
    }

    // Calculate remaining length
    let remaining_len = 2 + packet.return_codes.len(); // Packet ID + return codes

    // Allocate buffer
    let total_size = 1 + varint_len(remaining_len as u32) + remaining_len;
    let mut buf = BytesMut::with_capacity(total_size);

    // Fixed header: SUBACK = 0x90
    write_header(&mut buf, 0x90, remaining_len)?;

    // Variable header
    buf.put_u16(packet.packet_id);

    // Payload
    for code in &packet.return_codes {
        buf.put_u8(code.to_u8());
    }

    Ok(buf)
}

/// Encode UNSUBSCRIBE packet
fn encode_unsubscribe(packet: &Unsubscribe) -> EncodeResult {
    if packet.topic_filters.is_empty() {
        return Err(EncodeError::InvalidState(
            "UNSUBSCRIBE must have at least one topic filter".into(),
        ));
    }

    // Calculate remaining length
    let mut remaining_len = 2; // Packet ID

    for filter in &packet.topic_filters {
        remaining_len += string_size(filter);
    }

    // Allocate buffer
    let total_size = 1 + varint_len(remaining_len as u32) + remaining_len;
    let mut buf = BytesMut::with_capacity(total_size);

    // Fixed header: UNSUBSCRIBE = 0xA2 (flags = 0010)
    write_header(&mut buf, 0xA2, remaining_len)?;

    // Variable header
    buf.put_u16(packet.packet_id);

    // Payload
    for filter in &packet.topic_filters {
        encode_string(&mut buf, filter);
    }

    Ok(buf)
}

/// Encode UNSUBACK packet
fn encode_unsuback(packet: &UnsubAck) -> EncodeResult {
    let mut buf = BytesMut::with_capacity(4);

    // Fixed header: UNSUBACK = 0xB0, remaining length = 2
    buf.put_u8(0xB0);
    buf.put_u8(0x02);
    buf.put_u16(packet.packet_id);

    Ok(buf)
}

/// Encode PINGREQ packet
fn encode_pingreq() -> EncodeResult {
    let mut buf = BytesMut::with_capacity(2);

    // Fixed header: PINGREQ = 0xC0, remaining length = 0
    buf.put_u8(0xC0);
    buf.put_u8(0x00);

    Ok(buf)
}

/// Encode PINGRESP packet
fn encode_pingresp() -> EncodeResult {
    let mut buf = BytesMut::with_capacity(2);

    // Fixed header: PINGRESP = 0xD0, remaining length = 0
    buf.put_u8(0xD0);
    buf.put_u8(0x00);

    Ok(buf)
}

/// Encode DISCONNECT packet
fn encode_disconnect() -> EncodeResult {
    let mut buf = BytesMut::with_capacity(2);

    // Fixed header: DISCONNECT = 0xE0, remaining length = 0
    buf.put_u8(0xE0);
    buf.put_u8(0x00);

    Ok(buf)
}
