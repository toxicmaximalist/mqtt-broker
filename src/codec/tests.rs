//! Codec tests

use bytes::Bytes;

use super::*;

// ============================================================================
// CONNECT tests
// ============================================================================

#[test]
fn test_connect_minimal() {
    let packet = Connect::new("test-client");
    let encoded = encode_packet(&Packet::Connect(packet.clone())).unwrap();
    let (decoded, consumed) = decode_packet(&encoded).unwrap();

    assert_eq!(consumed, encoded.len());

    if let Packet::Connect(decoded) = decoded {
        assert_eq!(decoded.protocol_name, "MQTT");
        assert_eq!(decoded.protocol_level, 4);
        assert_eq!(decoded.client_id, "test-client");
        assert!(decoded.flags.clean_session);
        assert_eq!(decoded.keep_alive, 60);
        assert!(decoded.will.is_none());
        assert!(decoded.username.is_none());
        assert!(decoded.password.is_none());
    } else {
        panic!("Expected CONNECT packet");
    }
}

#[test]
fn test_connect_with_will() {
    let packet = Connect::new("test-client")
        .will(
            "will/topic".to_string(),
            Bytes::from("goodbye"),
            QoS::AtLeastOnce,
            true,
        )
        .keep_alive(120);

    let encoded = encode_packet(&Packet::Connect(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Connect(decoded) = decoded {
        let will = decoded.will.unwrap();
        assert_eq!(will.topic, "will/topic");
        assert_eq!(will.message, Bytes::from("goodbye"));
        assert_eq!(will.qos, QoS::AtLeastOnce);
        assert!(will.retain);
        assert_eq!(decoded.keep_alive, 120);
    } else {
        panic!("Expected CONNECT packet");
    }
}

#[test]
fn test_connect_with_credentials() {
    let packet =
        Connect::new("test-client").credentials("user".to_string(), Some(Bytes::from("pass")));

    let encoded = encode_packet(&Packet::Connect(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Connect(decoded) = decoded {
        assert_eq!(decoded.username, Some("user".to_string()));
        assert_eq!(decoded.password, Some(Bytes::from("pass")));
    } else {
        panic!("Expected CONNECT packet");
    }
}

#[test]
fn test_connect_empty_client_id_clean_session() {
    let mut packet = Connect::new("");
    packet.flags.clean_session = true;

    let encoded = encode_packet(&Packet::Connect(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Connect(decoded) = decoded {
        assert_eq!(decoded.client_id, "");
        assert!(decoded.flags.clean_session);
    } else {
        panic!("Expected CONNECT packet");
    }
}

#[test]
fn test_connect_empty_client_id_persistent_session_fails() {
    // Manually construct bytes for empty client_id with clean_session=false
    let bytes: &[u8] = &[
        0x10, // CONNECT
        0x0C, // Remaining length
        0x00, 0x04, b'M', b'Q', b'T', b'T', // Protocol name
        0x04, // Protocol level
        0x00, // Connect flags (clean_session=false)
        0x00, 0x3C, // Keep alive
        0x00, 0x00, // Empty client ID
    ];

    let result = decode_packet(bytes);
    assert!(matches!(result, Err(DecodeError::ProtocolViolation(_))));
}

// ============================================================================
// CONNACK tests
// ============================================================================

#[test]
fn test_connack_accepted() {
    let packet = ConnAck::accepted(false);
    let encoded = encode_packet(&Packet::ConnAck(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::ConnAck(decoded) = decoded {
        assert!(!decoded.session_present);
        assert_eq!(decoded.return_code, ConnectReturnCode::Accepted);
    } else {
        panic!("Expected CONNACK packet");
    }
}

#[test]
fn test_connack_session_present() {
    let packet = ConnAck::accepted(true);
    let encoded = encode_packet(&Packet::ConnAck(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::ConnAck(decoded) = decoded {
        assert!(decoded.session_present);
    } else {
        panic!("Expected CONNACK packet");
    }
}

#[test]
fn test_connack_rejected() {
    let packet = ConnAck::rejected(ConnectReturnCode::NotAuthorized);
    let encoded = encode_packet(&Packet::ConnAck(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::ConnAck(decoded) = decoded {
        assert!(!decoded.session_present);
        assert_eq!(decoded.return_code, ConnectReturnCode::NotAuthorized);
    } else {
        panic!("Expected CONNACK packet");
    }
}

// ============================================================================
// PUBLISH tests
// ============================================================================

#[test]
fn test_publish_qos0() {
    let packet = Publish::new("test/topic", Bytes::from("hello world"));
    let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Publish(decoded) = decoded {
        assert_eq!(decoded.topic, "test/topic");
        assert_eq!(decoded.payload, Bytes::from("hello world"));
        assert_eq!(decoded.qos, QoS::AtMostOnce);
        assert!(decoded.packet_id.is_none());
        assert!(!decoded.dup);
        assert!(!decoded.retain);
    } else {
        panic!("Expected PUBLISH packet");
    }
}

#[test]
fn test_publish_qos1() {
    let packet = Publish::with_qos("test/topic", Bytes::from("hello"), QoS::AtLeastOnce, 1234);
    let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Publish(decoded) = decoded {
        assert_eq!(decoded.qos, QoS::AtLeastOnce);
        assert_eq!(decoded.packet_id, Some(1234));
    } else {
        panic!("Expected PUBLISH packet");
    }
}

#[test]
fn test_publish_qos2_with_flags() {
    let packet = Publish::with_qos("test/topic", Bytes::from("hello"), QoS::ExactlyOnce, 5678)
        .retain(true)
        .dup(true);

    let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Publish(decoded) = decoded {
        assert_eq!(decoded.qos, QoS::ExactlyOnce);
        assert_eq!(decoded.packet_id, Some(5678));
        assert!(decoded.retain);
        assert!(decoded.dup);
    } else {
        panic!("Expected PUBLISH packet");
    }
}

#[test]
fn test_publish_empty_payload() {
    let packet = Publish::new("test/topic", Bytes::new());
    let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Publish(decoded) = decoded {
        assert!(decoded.payload.is_empty());
    } else {
        panic!("Expected PUBLISH packet");
    }
}

#[test]
fn test_publish_wildcard_topic_fails() {
    // Manually construct bytes with wildcard in topic
    let bytes: &[u8] = &[
        0x30, // PUBLISH QoS 0
        0x09, // Remaining length
        0x00, 0x05, b't', b'e', b's', b't', b'+', // Topic with wildcard
        b'h', b'i', // Payload
    ];

    let result = decode_packet(bytes);
    assert!(matches!(result, Err(DecodeError::InvalidTopicName(_))));
}

// ============================================================================
// PUBACK/PUBREC/PUBREL/PUBCOMP tests
// ============================================================================

#[test]
fn test_puback() {
    let packet = PubAck { packet_id: 1234 };
    let encoded = encode_packet(&Packet::PubAck(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    assert_eq!(decoded, Packet::PubAck(PubAck { packet_id: 1234 }));
}

#[test]
fn test_pubrec() {
    let packet = PubRec { packet_id: 5678 };
    let encoded = encode_packet(&Packet::PubRec(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    assert_eq!(decoded, Packet::PubRec(PubRec { packet_id: 5678 }));
}

#[test]
fn test_pubrel() {
    let packet = PubRel { packet_id: 9999 };
    let encoded = encode_packet(&Packet::PubRel(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    assert_eq!(decoded, Packet::PubRel(PubRel { packet_id: 9999 }));
}

#[test]
fn test_pubcomp() {
    let packet = PubComp { packet_id: 42 };
    let encoded = encode_packet(&Packet::PubComp(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    assert_eq!(decoded, Packet::PubComp(PubComp { packet_id: 42 }));
}

#[test]
fn test_zero_packet_id_fails() {
    // PUBACK with packet_id = 0
    let bytes: &[u8] = &[0x40, 0x02, 0x00, 0x00];
    let result = decode_packet(bytes);
    assert!(matches!(result, Err(DecodeError::ZeroPacketId)));
}

// ============================================================================
// SUBSCRIBE tests
// ============================================================================

#[test]
fn test_subscribe_single() {
    let packet = Subscribe::single(1, "test/topic", QoS::AtLeastOnce);
    let encoded = encode_packet(&Packet::Subscribe(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Subscribe(decoded) = decoded {
        assert_eq!(decoded.packet_id, 1);
        assert_eq!(decoded.subscriptions.len(), 1);
        assert_eq!(decoded.subscriptions[0].topic_filter, "test/topic");
        assert_eq!(decoded.subscriptions[0].qos, QoS::AtLeastOnce);
    } else {
        panic!("Expected SUBSCRIBE packet");
    }
}

#[test]
fn test_subscribe_multiple() {
    let packet = Subscribe::new(
        100,
        vec![
            Subscription {
                topic_filter: "topic/a".to_string(),
                qos: QoS::AtMostOnce,
            },
            Subscription {
                topic_filter: "topic/b".to_string(),
                qos: QoS::ExactlyOnce,
            },
            Subscription {
                topic_filter: "topic/#".to_string(),
                qos: QoS::AtLeastOnce,
            },
        ],
    );

    let encoded = encode_packet(&Packet::Subscribe(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Subscribe(decoded) = decoded {
        assert_eq!(decoded.subscriptions.len(), 3);
        assert_eq!(decoded.subscriptions[2].topic_filter, "topic/#");
    } else {
        panic!("Expected SUBSCRIBE packet");
    }
}

#[test]
fn test_subscribe_empty_fails() {
    let packet = Subscribe::new(1, vec![]);
    let result = encode_packet(&Packet::Subscribe(packet));
    assert!(matches!(result, Err(EncodeError::InvalidState(_))));
}

// ============================================================================
// SUBACK tests
// ============================================================================

#[test]
fn test_suback() {
    let packet = SubAck {
        packet_id: 100,
        return_codes: vec![
            SubAckCode::SuccessQoS0,
            SubAckCode::SuccessQoS1,
            SubAckCode::Failure,
        ],
    };

    let encoded = encode_packet(&Packet::SubAck(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::SubAck(decoded) = decoded {
        assert_eq!(decoded.packet_id, 100);
        assert_eq!(decoded.return_codes.len(), 3);
        assert_eq!(decoded.return_codes[0], SubAckCode::SuccessQoS0);
        assert_eq!(decoded.return_codes[2], SubAckCode::Failure);
    } else {
        panic!("Expected SUBACK packet");
    }
}

// ============================================================================
// UNSUBSCRIBE tests
// ============================================================================

#[test]
fn test_unsubscribe() {
    let packet = Unsubscribe::new(
        200,
        vec!["topic/a".to_string(), "topic/b".to_string()],
    );

    let encoded = encode_packet(&Packet::Unsubscribe(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Unsubscribe(decoded) = decoded {
        assert_eq!(decoded.packet_id, 200);
        assert_eq!(decoded.topic_filters.len(), 2);
    } else {
        panic!("Expected UNSUBSCRIBE packet");
    }
}

// ============================================================================
// UNSUBACK tests
// ============================================================================

#[test]
fn test_unsuback() {
    let packet = UnsubAck { packet_id: 300 };
    let encoded = encode_packet(&Packet::UnsubAck(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    assert_eq!(decoded, Packet::UnsubAck(UnsubAck { packet_id: 300 }));
}

// ============================================================================
// PING tests
// ============================================================================

#[test]
fn test_pingreq() {
    let encoded = encode_packet(&Packet::PingReq).unwrap();
    assert_eq!(&encoded[..], &[0xC0, 0x00]);

    let (decoded, consumed) = decode_packet(&encoded).unwrap();
    assert_eq!(decoded, Packet::PingReq);
    assert_eq!(consumed, 2);
}

#[test]
fn test_pingresp() {
    let encoded = encode_packet(&Packet::PingResp).unwrap();
    assert_eq!(&encoded[..], &[0xD0, 0x00]);

    let (decoded, _) = decode_packet(&encoded).unwrap();
    assert_eq!(decoded, Packet::PingResp);
}

// ============================================================================
// DISCONNECT tests
// ============================================================================

#[test]
fn test_disconnect() {
    let encoded = encode_packet(&Packet::Disconnect).unwrap();
    assert_eq!(&encoded[..], &[0xE0, 0x00]);

    let (decoded, _) = decode_packet(&encoded).unwrap();
    assert_eq!(decoded, Packet::Disconnect);
}

// ============================================================================
// Error handling tests
// ============================================================================

#[test]
fn test_incomplete_packet() {
    // Only fixed header, no payload
    let bytes: &[u8] = &[0x10, 0x10]; // CONNECT with remaining length 16, but no data
    let result = decode_packet(bytes);
    assert!(matches!(result, Err(DecodeError::Incomplete { .. })));
}

#[test]
fn test_invalid_packet_type() {
    let bytes: &[u8] = &[0x00, 0x00]; // Packet type 0 is invalid
    let result = decode_packet(bytes);
    assert!(matches!(result, Err(DecodeError::InvalidPacketType(0))));

    let bytes: &[u8] = &[0xF0, 0x00]; // Packet type 15 is invalid
    let result = decode_packet(bytes);
    assert!(matches!(result, Err(DecodeError::InvalidPacketType(15))));
}

#[test]
fn test_invalid_flags() {
    // CONNECT with non-zero flags (must be 0)
    let bytes: &[u8] = &[0x11, 0x00];
    let result = decode_packet(bytes);
    assert!(matches!(result, Err(DecodeError::InvalidFlags { .. })));
}

#[test]
fn test_decode_multiple_packets() {
    // Encode two packets into the same buffer
    let mut buf = encode_packet(&Packet::PingReq).unwrap();
    buf.extend_from_slice(&encode_packet(&Packet::PingResp).unwrap());

    // Decode first packet
    let (packet1, consumed1) = decode_packet(&buf).unwrap();
    assert_eq!(packet1, Packet::PingReq);
    assert_eq!(consumed1, 2);

    // Decode second packet
    let (packet2, consumed2) = decode_packet(&buf[consumed1..]).unwrap();
    assert_eq!(packet2, Packet::PingResp);
    assert_eq!(consumed2, 2);
}

// ============================================================================
// QoS tests
// ============================================================================

#[test]
fn test_qos_from_u8() {
    assert_eq!(QoS::from_u8(0), Some(QoS::AtMostOnce));
    assert_eq!(QoS::from_u8(1), Some(QoS::AtLeastOnce));
    assert_eq!(QoS::from_u8(2), Some(QoS::ExactlyOnce));
    assert_eq!(QoS::from_u8(3), None);
}

#[test]
fn test_qos_min() {
    assert_eq!(QoS::AtMostOnce.min(QoS::ExactlyOnce), QoS::AtMostOnce);
    assert_eq!(QoS::ExactlyOnce.min(QoS::AtLeastOnce), QoS::AtLeastOnce);
    assert_eq!(QoS::AtLeastOnce.min(QoS::AtLeastOnce), QoS::AtLeastOnce);
}

// ============================================================================
// Connect flags tests
// ============================================================================

#[test]
fn test_connect_flags_roundtrip() {
    let flags = ConnectFlags {
        clean_session: true,
        will: true,
        will_qos: QoS::ExactlyOnce,
        will_retain: true,
        username: true,
        password: true,
    };

    let byte = flags.to_byte();
    let parsed = ConnectFlags::from_byte(byte).unwrap();

    assert_eq!(parsed.clean_session, flags.clean_session);
    assert_eq!(parsed.will, flags.will);
    assert_eq!(parsed.will_qos, flags.will_qos);
    assert_eq!(parsed.will_retain, flags.will_retain);
    assert_eq!(parsed.username, flags.username);
    assert_eq!(parsed.password, flags.password);
}

#[test]
fn test_connect_flags_reserved_bit() {
    // Reserved bit 0 must be 0
    let result = ConnectFlags::from_byte(0x01);
    assert!(result.is_none());
}

#[test]
fn test_connect_flags_validation() {
    // Will retain without will flag
    let flags = ConnectFlags {
        will: false,
        will_retain: true,
        ..Default::default()
    };
    assert!(!flags.validate());

    // Password without username
    let flags = ConnectFlags {
        password: true,
        username: false,
        ..Default::default()
    };
    assert!(!flags.validate());
}
