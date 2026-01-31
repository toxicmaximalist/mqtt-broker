//! Property-based tests for MQTT codec

use bytes::Bytes;
use proptest::prelude::*;

use mqtt_broker::codec::*;

// Strategy for generating valid QoS values
fn qos_strategy() -> impl Strategy<Value = QoS> {
    prop_oneof![
        Just(QoS::AtMostOnce),
        Just(QoS::AtLeastOnce),
        Just(QoS::ExactlyOnce),
    ]
}

// Strategy for generating valid topic names (no wildcards for PUBLISH)
fn topic_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_/]{1,100}"
        .prop_filter("no wildcards", |s| !s.contains('+') && !s.contains('#'))
}

// Strategy for generating valid topic filters (can have wildcards)
fn topic_filter_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9_/]{1,50}",                    // Simple topic
        "[a-zA-Z0-9_/]{1,20}/\\+".prop_map(|s| s), // Single-level wildcard
        "[a-zA-Z0-9_/]{1,20}/#".prop_map(|s| s),  // Multi-level wildcard
    ]
}

// Strategy for generating valid client IDs
fn client_id_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{0,23}"
}

// Strategy for payloads
fn payload_strategy() -> impl Strategy<Value = Bytes> {
    prop::collection::vec(any::<u8>(), 0..1000).prop_map(Bytes::from)
}

// Strategy for packet IDs (non-zero)
fn packet_id_strategy() -> impl Strategy<Value = u16> {
    1..=u16::MAX
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    // ========================================================================
    // CONNECT roundtrip tests
    // ========================================================================

    #[test]
    fn connect_roundtrip(
        client_id in client_id_strategy(),
        clean_session in any::<bool>(),
        keep_alive in any::<u16>(),
    ) {
        // Skip empty client_id with persistent session (protocol violation)
        prop_assume!(clean_session || !client_id.is_empty());

        let mut packet = Connect::new(client_id.clone());
        packet.flags.clean_session = clean_session;
        packet.keep_alive = keep_alive;

        let encoded = encode_packet(&Packet::Connect(packet.clone())).unwrap();
        let (decoded, consumed) = decode_packet(&encoded).unwrap();

        prop_assert_eq!(consumed, encoded.len());

        if let Packet::Connect(decoded) = decoded {
            prop_assert_eq!(decoded.client_id, client_id);
            prop_assert_eq!(decoded.flags.clean_session, clean_session);
            prop_assert_eq!(decoded.keep_alive, keep_alive);
        } else {
            prop_assert!(false, "Expected CONNECT packet");
        }
    }

    #[test]
    fn connect_with_credentials_roundtrip(
        client_id in "[a-zA-Z0-9]{1,20}",
        username in "[a-zA-Z0-9_]{1,50}",
        password in prop::collection::vec(any::<u8>(), 0..100),
    ) {
        let packet = Connect::new(client_id.clone())
            .credentials(username.clone(), Some(Bytes::from(password.clone())));

        let encoded = encode_packet(&Packet::Connect(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        if let Packet::Connect(decoded) = decoded {
            prop_assert_eq!(decoded.username, Some(username));
            prop_assert_eq!(decoded.password, Some(Bytes::from(password)));
        } else {
            prop_assert!(false, "Expected CONNECT packet");
        }
    }

    // ========================================================================
    // PUBLISH roundtrip tests
    // ========================================================================

    #[test]
    fn publish_qos0_roundtrip(
        topic in topic_name_strategy(),
        payload in payload_strategy(),
        retain in any::<bool>(),
    ) {
        let packet = Publish::new(topic.clone(), payload.clone()).retain(retain);

        let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
        let (decoded, consumed) = decode_packet(&encoded).unwrap();

        prop_assert_eq!(consumed, encoded.len());

        if let Packet::Publish(decoded) = decoded {
            prop_assert_eq!(decoded.topic, topic);
            prop_assert_eq!(decoded.payload, payload);
            prop_assert_eq!(decoded.retain, retain);
            prop_assert_eq!(decoded.qos, QoS::AtMostOnce);
            prop_assert!(decoded.packet_id.is_none());
        } else {
            prop_assert!(false, "Expected PUBLISH packet");
        }
    }

    #[test]
    fn publish_qos1_roundtrip(
        topic in topic_name_strategy(),
        payload in payload_strategy(),
        packet_id in packet_id_strategy(),
        dup in any::<bool>(),
        retain in any::<bool>(),
    ) {
        let packet = Publish::with_qos(topic.clone(), payload.clone(), QoS::AtLeastOnce, packet_id)
            .dup(dup)
            .retain(retain);

        let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        if let Packet::Publish(decoded) = decoded {
            prop_assert_eq!(decoded.topic, topic);
            prop_assert_eq!(decoded.payload, payload);
            prop_assert_eq!(decoded.qos, QoS::AtLeastOnce);
            prop_assert_eq!(decoded.packet_id, Some(packet_id));
            prop_assert_eq!(decoded.dup, dup);
            prop_assert_eq!(decoded.retain, retain);
        } else {
            prop_assert!(false, "Expected PUBLISH packet");
        }
    }

    #[test]
    fn publish_qos2_roundtrip(
        topic in topic_name_strategy(),
        payload in payload_strategy(),
        packet_id in packet_id_strategy(),
    ) {
        let packet = Publish::with_qos(topic.clone(), payload.clone(), QoS::ExactlyOnce, packet_id);

        let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        if let Packet::Publish(decoded) = decoded {
            prop_assert_eq!(decoded.qos, QoS::ExactlyOnce);
            prop_assert_eq!(decoded.packet_id, Some(packet_id));
        } else {
            prop_assert!(false, "Expected PUBLISH packet");
        }
    }

    // ========================================================================
    // QoS handshake packet roundtrips
    // ========================================================================

    #[test]
    fn puback_roundtrip(packet_id in packet_id_strategy()) {
        let packet = PubAck { packet_id };
        let encoded = encode_packet(&Packet::PubAck(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        prop_assert_eq!(decoded, Packet::PubAck(PubAck { packet_id }));
    }

    #[test]
    fn pubrec_roundtrip(packet_id in packet_id_strategy()) {
        let packet = PubRec { packet_id };
        let encoded = encode_packet(&Packet::PubRec(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        prop_assert_eq!(decoded, Packet::PubRec(PubRec { packet_id }));
    }

    #[test]
    fn pubrel_roundtrip(packet_id in packet_id_strategy()) {
        let packet = PubRel { packet_id };
        let encoded = encode_packet(&Packet::PubRel(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        prop_assert_eq!(decoded, Packet::PubRel(PubRel { packet_id }));
    }

    #[test]
    fn pubcomp_roundtrip(packet_id in packet_id_strategy()) {
        let packet = PubComp { packet_id };
        let encoded = encode_packet(&Packet::PubComp(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        prop_assert_eq!(decoded, Packet::PubComp(PubComp { packet_id }));
    }

    // ========================================================================
    // SUBSCRIBE/SUBACK roundtrips
    // ========================================================================

    #[test]
    fn subscribe_roundtrip(
        packet_id in packet_id_strategy(),
        topics in prop::collection::vec(
            (topic_filter_strategy(), qos_strategy()),
            1..10
        ),
    ) {
        let subscriptions: Vec<Subscription> = topics
            .into_iter()
            .map(|(topic_filter, qos)| Subscription { topic_filter, qos })
            .collect();

        let packet = Subscribe::new(packet_id, subscriptions.clone());

        let encoded = encode_packet(&Packet::Subscribe(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        if let Packet::Subscribe(decoded) = decoded {
            prop_assert_eq!(decoded.packet_id, packet_id);
            prop_assert_eq!(decoded.subscriptions.len(), subscriptions.len());
            for (expected, actual) in subscriptions.iter().zip(decoded.subscriptions.iter()) {
                prop_assert_eq!(&expected.topic_filter, &actual.topic_filter);
                prop_assert_eq!(expected.qos, actual.qos);
            }
        } else {
            prop_assert!(false, "Expected SUBSCRIBE packet");
        }
    }

    #[test]
    fn suback_roundtrip(
        packet_id in packet_id_strategy(),
        codes in prop::collection::vec(
            prop_oneof![
                Just(SubAckCode::SuccessQoS0),
                Just(SubAckCode::SuccessQoS1),
                Just(SubAckCode::SuccessQoS2),
                Just(SubAckCode::Failure),
            ],
            1..10
        ),
    ) {
        let packet = SubAck {
            packet_id,
            return_codes: codes.clone(),
        };

        let encoded = encode_packet(&Packet::SubAck(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        if let Packet::SubAck(decoded) = decoded {
            prop_assert_eq!(decoded.packet_id, packet_id);
            prop_assert_eq!(decoded.return_codes, codes);
        } else {
            prop_assert!(false, "Expected SUBACK packet");
        }
    }

    // ========================================================================
    // UNSUBSCRIBE/UNSUBACK roundtrips
    // ========================================================================

    #[test]
    fn unsubscribe_roundtrip(
        packet_id in packet_id_strategy(),
        topics in prop::collection::vec(topic_filter_strategy(), 1..10),
    ) {
        let packet = Unsubscribe::new(packet_id, topics.clone());

        let encoded = encode_packet(&Packet::Unsubscribe(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        if let Packet::Unsubscribe(decoded) = decoded {
            prop_assert_eq!(decoded.packet_id, packet_id);
            prop_assert_eq!(decoded.topic_filters, topics);
        } else {
            prop_assert!(false, "Expected UNSUBSCRIBE packet");
        }
    }

    #[test]
    fn unsuback_roundtrip(packet_id in packet_id_strategy()) {
        let packet = UnsubAck { packet_id };
        let encoded = encode_packet(&Packet::UnsubAck(packet)).unwrap();
        let (decoded, _) = decode_packet(&encoded).unwrap();

        prop_assert_eq!(decoded, Packet::UnsubAck(UnsubAck { packet_id }));
    }

    // ========================================================================
    // Variable integer encoding
    // ========================================================================

    #[test]
    fn varint_roundtrip(value in 0u32..=268_435_455u32) {
        let encoded = encode_variable_int(value);
        let (decoded, len) = decode_variable_int(&encoded).unwrap();

        prop_assert_eq!(decoded, value);
        prop_assert_eq!(len, encoded.len());
        prop_assert!(len <= 4);
    }

    // ========================================================================
    // Connect flags
    // ========================================================================

    #[test]
    fn connect_flags_roundtrip(
        clean_session in any::<bool>(),
        will in any::<bool>(),
        will_qos in qos_strategy(),
        will_retain in any::<bool>(),
        username in any::<bool>(),
        password in any::<bool>(),
    ) {
        // Apply protocol constraints
        let will_qos = if will { will_qos } else { QoS::AtMostOnce };
        let will_retain = if will { will_retain } else { false };
        let password = if username { password } else { false };

        let flags = ConnectFlags {
            clean_session,
            will,
            will_qos,
            will_retain,
            username,
            password,
        };

        let byte = flags.to_byte();
        let parsed = ConnectFlags::from_byte(byte);

        prop_assert!(parsed.is_some());
        let parsed = parsed.unwrap();

        prop_assert_eq!(parsed.clean_session, clean_session);
        prop_assert_eq!(parsed.will, will);
        prop_assert_eq!(parsed.will_qos, will_qos);
        prop_assert_eq!(parsed.will_retain, will_retain);
        prop_assert_eq!(parsed.username, username);
        prop_assert_eq!(parsed.password, password);
    }
}

// ============================================================================
// Non-proptest tests for edge cases
// ============================================================================

#[test]
fn test_large_payload() {
    // Test with ~100KB payload
    let payload = Bytes::from(vec![0xAB; 100_000]);
    let packet = Publish::new("test/large", payload.clone());

    let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Publish(decoded) = decoded {
        assert_eq!(decoded.payload.len(), 100_000);
        assert_eq!(decoded.payload, payload);
    } else {
        panic!("Expected PUBLISH packet");
    }
}

#[test]
fn test_many_subscriptions() {
    // Test with 100 subscriptions
    let subscriptions: Vec<Subscription> = (0..100)
        .map(|i| Subscription {
            topic_filter: format!("topic/{}/sub", i),
            qos: QoS::from_u8((i % 3) as u8).unwrap(),
        })
        .collect();

    let packet = Subscribe::new(1, subscriptions.clone());

    let encoded = encode_packet(&Packet::Subscribe(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Subscribe(decoded) = decoded {
        assert_eq!(decoded.subscriptions.len(), 100);
    } else {
        panic!("Expected SUBSCRIBE packet");
    }
}

#[test]
fn test_unicode_topic() {
    // MQTT allows UTF-8 topics
    let topic = "sensors/温度/北京";
    let packet = Publish::new(topic, Bytes::from("25°C"));

    let encoded = encode_packet(&Packet::Publish(packet)).unwrap();
    let (decoded, _) = decode_packet(&encoded).unwrap();

    if let Packet::Publish(decoded) = decoded {
        assert_eq!(decoded.topic, topic);
    } else {
        panic!("Expected PUBLISH packet");
    }
}

#[test]
fn test_streaming_decode() {
    // Simulate streaming: feed bytes one at a time
    let packet = Publish::new("test/stream", Bytes::from("hello"));
    let encoded = encode_packet(&Packet::Publish(packet)).unwrap();

    for i in 0..encoded.len() - 1 {
        let partial = &encoded[..i + 1];
        let result = decode_packet(partial);
        assert!(
            matches!(result, Err(DecodeError::Incomplete { .. })),
            "Expected Incomplete for {} bytes, got {:?}",
            i + 1,
            result
        );
    }

    // Full packet should decode
    let (decoded, consumed) = decode_packet(&encoded).unwrap();
    assert_eq!(consumed, encoded.len());
    assert!(matches!(decoded, Packet::Publish(_)));
}
