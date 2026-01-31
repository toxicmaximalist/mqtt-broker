//! Session module tests

use std::time::Duration;

use bytes::Bytes;

use super::*;
use crate::codec::QoS;
use super::qos::{QoSError, QoSConfig};
use super::session::QueuedMessage;

// ============================================================================
// Connection state machine tests
// ============================================================================

#[test]
fn test_connection_state_flow() {
    let mut conn = ConnectionStateMachine::new();

    assert_eq!(conn.state(), ConnectionState::AwaitingConnect);
    assert!(!conn.is_connected());

    // Receive CONNECT
    conn.on_connect("client-1".to_string(), true, 60).unwrap();
    assert_eq!(conn.state(), ConnectionState::Authenticating);
    assert_eq!(conn.client_id(), Some("client-1"));

    // Authentication succeeds
    conn.on_authenticated().unwrap();
    assert_eq!(conn.state(), ConnectionState::Connected);
    assert!(conn.is_connected());

    // Client disconnects
    conn.on_disconnect().unwrap();
    assert_eq!(conn.state(), ConnectionState::Disconnecting);

    // Connection closed
    conn.on_closed();
    assert_eq!(conn.state(), ConnectionState::Closed);
    assert!(conn.is_closed());
}

#[test]
fn test_connection_double_connect_error() {
    let mut conn = ConnectionStateMachine::new();

    conn.on_connect("client-1".to_string(), true, 60).unwrap();
    conn.on_authenticated().unwrap();

    // Second CONNECT should fail
    let result = conn.on_connect("client-2".to_string(), true, 60);
    assert!(result.is_err());
}

#[test]
fn test_keep_alive_timeout() {
    let mut conn = ConnectionStateMachine::new();
    conn.on_connect("client-1".to_string(), true, 0).unwrap(); // keep_alive = 0 (disabled)

    // With keep_alive=0, should never timeout
    assert!(!conn.is_keep_alive_expired());

    // Create new with 1 second keep-alive
    let mut conn2 = ConnectionStateMachine::new();
    conn2.on_connect("client-2".to_string(), true, 1).unwrap();
    
    // Should not be expired immediately
    assert!(!conn2.is_keep_alive_expired());
}

// ============================================================================
// Packet ID allocator tests
// ============================================================================

#[test]
fn test_packet_id_basic() {
    let mut alloc = PacketIdAllocator::new();

    let id1 = alloc.allocate().unwrap();
    let id2 = alloc.allocate().unwrap();

    assert_ne!(id1, id2);
    assert!(alloc.is_in_use(id1));
    assert!(alloc.is_in_use(id2));

    alloc.release(id1);
    assert!(!alloc.is_in_use(id1));
}

// ============================================================================
// QoS 1 state machine tests
// ============================================================================

#[test]
fn test_qos1_success_flow() {
    let pending = PendingPublish::new(
        1,
        "test/topic".to_string(),
        Bytes::from("hello"),
        QoS::AtLeastOnce,
        false,
    );

    let state = QoS1State::new(pending);
    assert!(!state.is_complete());
    assert!(state.pending().is_some());

    // Receive PUBACK
    let state = state.on_puback(1).unwrap();
    assert!(state.is_complete());
    assert!(state.pending().is_none());
}

#[test]
fn test_qos1_wrong_packet_id() {
    let pending = PendingPublish::new(
        1,
        "test/topic".to_string(),
        Bytes::from("hello"),
        QoS::AtLeastOnce,
        false,
    );

    let state = QoS1State::new(pending);
    let result = state.on_puback(999);

    assert!(matches!(result, Err(QoSError::PacketIdMismatch { .. })));
}

// ============================================================================
// QoS 2 state machine tests
// ============================================================================

#[test]
fn test_qos2_outbound_success_flow() {
    let pending = PendingPublish::new(
        1,
        "test/topic".to_string(),
        Bytes::from("hello"),
        QoS::ExactlyOnce,
        false,
    );

    // Start with PUBLISH sent
    let state = QoS2OutboundState::new(pending);
    assert!(!state.is_complete());

    // Receive PUBREC
    let state = state.on_pubrec(1).unwrap();
    assert!(matches!(state, QoS2OutboundState::AwaitingPubComp { .. }));

    // Receive PUBCOMP
    let state = state.on_pubcomp(1).unwrap();
    assert!(state.is_complete());
}

#[test]
fn test_qos2_inbound_success_flow() {
    // Receive PUBLISH, create state
    let state = QoS2InboundState::new(42);
    assert!(!state.is_complete());

    // Receive PUBREL
    let state = state.on_pubrel(42).unwrap();
    assert!(state.is_complete());
}

// ============================================================================
// Session tests
// ============================================================================

#[test]
fn test_session_subscriptions() {
    let mut session = Session::new(
        "client-1".to_string(),
        true,
        SessionConfig::default(),
    );

    // Add subscriptions
    session.subscribe("test/+/topic".to_string(), QoS::AtLeastOnce);
    session.subscribe("sensors/#".to_string(), QoS::ExactlyOnce);

    assert!(session.is_subscribed("test/+/topic"));
    assert!(session.is_subscribed("sensors/#"));
    assert!(!session.is_subscribed("other/topic"));

    assert_eq!(session.subscriptions().len(), 2);

    // Remove subscription
    assert!(session.unsubscribe("test/+/topic"));
    assert!(!session.is_subscribed("test/+/topic"));
}

#[test]
fn test_session_qos1_flow() {
    let mut session = Session::new(
        "client-1".to_string(),
        true,
        SessionConfig::default(),
    );

    let packet_id = session.allocate_packet_id().unwrap();
    let pending = PendingPublish::new(
        packet_id,
        "test/topic".to_string(),
        Bytes::from("hello"),
        QoS::AtLeastOnce,
        false,
    );

    // Start publish
    session.start_qos1_publish(pending).unwrap();
    assert_eq!(session.inflight_count(), 1);

    // Receive PUBACK
    session.on_puback(packet_id).unwrap();
    assert_eq!(session.inflight_count(), 0);
}

#[test]
fn test_session_qos2_outbound_flow() {
    let mut session = Session::new(
        "client-1".to_string(),
        true,
        SessionConfig::default(),
    );

    let packet_id = session.allocate_packet_id().unwrap();
    let pending = PendingPublish::new(
        packet_id,
        "test/topic".to_string(),
        Bytes::from("hello"),
        QoS::ExactlyOnce,
        false,
    );

    // Start publish
    session.start_qos2_publish(pending).unwrap();
    assert_eq!(session.inflight_count(), 1);

    // Receive PUBREC
    let needs_pubrel = session.on_pubrec(packet_id).unwrap();
    assert!(needs_pubrel);
    assert_eq!(session.inflight_count(), 1);

    // Receive PUBCOMP
    session.on_pubcomp(packet_id).unwrap();
    assert_eq!(session.inflight_count(), 0);
}

#[test]
fn test_session_qos2_inbound_flow() {
    let mut session = Session::new(
        "client-1".to_string(),
        true,
        SessionConfig::default(),
    );

    // Receive PUBLISH with QoS 2
    let is_new = session.start_qos2_receive(42);
    assert!(is_new);

    // Duplicate PUBLISH
    let is_new = session.start_qos2_receive(42);
    assert!(!is_new);

    // Receive PUBREL
    let result = session.on_pubrel(42).unwrap();
    assert!(result);
}

#[test]
fn test_session_offline_queue() {
    let config = SessionConfig {
        max_offline_messages: 3,
        ..Default::default()
    };

    let mut session = Session::new(
        "client-1".to_string(),
        false, // persistent session
        config,
    );

    // Queue messages
    for i in 0..5 {
        session.queue_message(QueuedMessage {
            topic: format!("topic/{}", i),
            payload: Bytes::from("test"),
            qos: QoS::AtMostOnce,
            retain: false,
            queued_at: std::time::Instant::now(),
        });
    }

    // Should only have 3 (oldest dropped)
    assert_eq!(session.queued_message_count(), 3);

    // Dequeue
    let msg = session.dequeue_message().unwrap();
    assert_eq!(msg.topic, "topic/2"); // 0 and 1 were dropped

    assert_eq!(session.queued_message_count(), 2);
}

#[test]
fn test_session_clear() {
    let mut session = Session::new(
        "client-1".to_string(),
        true,
        SessionConfig::default(),
    );

    session.subscribe("test/topic".to_string(), QoS::AtMostOnce);
    session.queue_message(QueuedMessage {
        topic: "topic".to_string(),
        payload: Bytes::new(),
        qos: QoS::AtMostOnce,
        retain: false,
        queued_at: std::time::Instant::now(),
    });

    session.clear();

    assert_eq!(session.subscriptions().len(), 0);
    assert_eq!(session.queued_message_count(), 0);
}

#[test]
fn test_session_stats() {
    let mut session = Session::new(
        "client-1".to_string(),
        true,
        SessionConfig::default(),
    );

    session.subscribe("topic/a".to_string(), QoS::AtMostOnce);
    session.subscribe("topic/b".to_string(), QoS::AtLeastOnce);

    let stats = session.stats();
    assert_eq!(stats.subscription_count, 2);
    assert_eq!(stats.qos1_inflight, 0);
    assert_eq!(stats.offline_queue_size, 0);
}

// ============================================================================
// Retry logic tests
// ============================================================================

#[test]
fn test_pending_publish_retry() {
    let mut pending = PendingPublish::new(
        1,
        "test/topic".to_string(),
        Bytes::from("hello"),
        QoS::AtLeastOnce,
        false,
    );

    let config = QoSConfig {
        retry_interval: Duration::from_millis(10),
        max_retries: 3,
        exponential_backoff: false,
    };

    // Should not retry immediately
    assert!(!pending.should_retry(&config));

    // Wait a bit
    std::thread::sleep(Duration::from_millis(15));
    assert!(pending.should_retry(&config));

    // Mark as retried
    pending.mark_retried();
    assert_eq!(pending.attempts, 2);
    assert!(!pending.should_retry(&config));
}
