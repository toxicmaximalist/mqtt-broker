//! Tests for the router module.

use super::*;
use crate::codec::QoS;

mod registry_tests {
    use super::*;

    #[test]
    fn test_subscribe_and_get_subscribers() {
        let registry = SubscriptionRegistry::new();
        
        registry.subscribe("client1", "sensor/+/temp", QoS::AtLeastOnce).unwrap();
        registry.subscribe("client2", "sensor/#", QoS::ExactlyOnce).unwrap();
        registry.subscribe("client3", "sensor/room1/temp", QoS::AtMostOnce).unwrap();

        // Check matching
        let subs = registry.get_subscribers("sensor/room1/temp");
        assert_eq!(subs.len(), 3);

        // Only client2 should match this
        let subs = registry.get_subscribers("sensor/room1/humidity");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].0, "client2");
    }

    #[test]
    fn test_subscribe_updates_qos() {
        let registry = SubscriptionRegistry::new();
        
        registry.subscribe("client1", "test/topic", QoS::AtMostOnce).unwrap();
        registry.subscribe("client1", "test/topic", QoS::ExactlyOnce).unwrap();

        let subs = registry.get_client_subscriptions("client1");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].qos, QoS::ExactlyOnce);
    }

    #[test]
    fn test_unsubscribe() {
        let registry = SubscriptionRegistry::new();
        
        registry.subscribe("client1", "test/topic", QoS::AtLeastOnce).unwrap();
        assert!(registry.has_subscriptions("client1"));

        let removed = registry.unsubscribe("client1", "test/topic");
        assert!(removed);
        assert!(!registry.has_subscriptions("client1"));

        // Unsubscribing again should return false
        let removed = registry.unsubscribe("client1", "test/topic");
        assert!(!removed);
    }

    #[test]
    fn test_remove_client() {
        let registry = SubscriptionRegistry::new();
        
        registry.subscribe("client1", "test/a", QoS::AtMostOnce).unwrap();
        registry.subscribe("client1", "test/b", QoS::AtLeastOnce).unwrap();
        registry.subscribe("client1", "test/c", QoS::ExactlyOnce).unwrap();

        let removed = registry.remove_client("client1");
        assert_eq!(removed.len(), 3);
        assert!(!registry.has_subscriptions("client1"));
        assert_eq!(registry.subscription_count(), 0);
    }

    #[test]
    fn test_multiple_clients_same_filter() {
        let registry = SubscriptionRegistry::new();
        
        registry.subscribe("client1", "shared/topic", QoS::AtMostOnce).unwrap();
        registry.subscribe("client2", "shared/topic", QoS::AtLeastOnce).unwrap();
        registry.subscribe("client3", "shared/topic", QoS::ExactlyOnce).unwrap();

        let subs = registry.get_subscribers("shared/topic");
        assert_eq!(subs.len(), 3);

        // Remove one client
        registry.remove_client("client2");
        let subs = registry.get_subscribers("shared/topic");
        assert_eq!(subs.len(), 2);
    }

    #[test]
    fn test_highest_qos_for_multiple_matches() {
        let registry = SubscriptionRegistry::new();
        
        // Same client, overlapping filters with different QoS
        registry.subscribe("client1", "sensor/+/temp", QoS::AtMostOnce).unwrap();
        registry.subscribe("client1", "sensor/#", QoS::ExactlyOnce).unwrap();

        let subs = registry.get_subscribers("sensor/room1/temp");
        assert_eq!(subs.len(), 1);
        // Should get the higher QoS
        assert_eq!(subs[0].1, QoS::ExactlyOnce);
    }

    #[test]
    fn test_invalid_filter_rejected() {
        let registry = SubscriptionRegistry::new();
        
        // Invalid: + in middle of level
        assert!(registry.subscribe("client1", "test/a+b/topic", QoS::AtMostOnce).is_err());
        
        // Invalid: # not at end
        assert!(registry.subscribe("client1", "test/#/topic", QoS::AtMostOnce).is_err());
    }

    #[test]
    fn test_counts() {
        let registry = SubscriptionRegistry::new();
        
        registry.subscribe("client1", "a", QoS::AtMostOnce).unwrap();
        registry.subscribe("client1", "b", QoS::AtMostOnce).unwrap();
        registry.subscribe("client2", "a", QoS::AtMostOnce).unwrap();

        assert_eq!(registry.subscription_count(), 3);
        assert_eq!(registry.client_count(), 2);
    }
}

mod retain_tests {
    use super::*;
    use super::retain::RetainError;

    #[test]
    fn test_store_and_get() {
        let store = RetainStore::new();
        
        store.store("test/topic", b"hello".to_vec(), QoS::AtLeastOnce).unwrap();
        
        let msg = store.get("test/topic").unwrap();
        assert_eq!(msg.topic, "test/topic");
        assert_eq!(msg.payload, b"hello");
        assert_eq!(msg.qos, QoS::AtLeastOnce);
    }

    #[test]
    fn test_empty_payload_deletes() {
        let store = RetainStore::new();
        
        store.store("test/topic", b"hello".to_vec(), QoS::AtMostOnce).unwrap();
        assert!(store.get("test/topic").is_some());

        // Empty payload should delete
        store.store("test/topic", vec![], QoS::AtMostOnce).unwrap();
        assert!(store.get("test/topic").is_none());
    }

    #[test]
    fn test_update_retained_message() {
        let store = RetainStore::new();
        
        let is_new = store.store("test/topic", b"first".to_vec(), QoS::AtMostOnce).unwrap();
        assert!(is_new);

        let is_new = store.store("test/topic", b"second".to_vec(), QoS::ExactlyOnce).unwrap();
        assert!(!is_new);

        let msg = store.get("test/topic").unwrap();
        assert_eq!(msg.payload, b"second");
        assert_eq!(msg.qos, QoS::ExactlyOnce);
    }

    #[test]
    fn test_get_matching_wildcards() {
        let store = RetainStore::new();
        
        store.store("sensor/room1/temp", b"20".to_vec(), QoS::AtMostOnce).unwrap();
        store.store("sensor/room1/humidity", b"50".to_vec(), QoS::AtMostOnce).unwrap();
        store.store("sensor/room2/temp", b"22".to_vec(), QoS::AtMostOnce).unwrap();
        store.store("other/topic", b"x".to_vec(), QoS::AtMostOnce).unwrap();

        // Match with +
        let matches = store.get_matching("sensor/+/temp");
        assert_eq!(matches.len(), 2);

        // Match with #
        let matches = store.get_matching("sensor/#");
        assert_eq!(matches.len(), 3);

        // Exact match
        let matches = store.get_matching("sensor/room1/temp");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_payload_size_limit() {
        let store = RetainStore::with_limits(100, 10); // 10 byte limit
        
        // Should fail - too large
        let result = store.store("topic", vec![0u8; 20], QoS::AtMostOnce);
        assert!(matches!(result, Err(RetainError::PayloadTooLarge { .. })));

        // Should succeed - within limit
        let result = store.store("topic", vec![0u8; 5], QoS::AtMostOnce);
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_count_limit() {
        let store = RetainStore::with_limits(2, 1024); // 2 message limit
        
        store.store("topic1", b"a".to_vec(), QoS::AtMostOnce).unwrap();
        store.store("topic2", b"b".to_vec(), QoS::AtMostOnce).unwrap();
        
        // Should fail - store full
        let result = store.store("topic3", b"c".to_vec(), QoS::AtMostOnce);
        assert!(matches!(result, Err(RetainError::StoreFull { .. })));

        // But updating existing should work
        let result = store.store("topic1", b"aa".to_vec(), QoS::AtMostOnce);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear() {
        let store = RetainStore::new();
        
        store.store("a", b"1".to_vec(), QoS::AtMostOnce).unwrap();
        store.store("b", b"2".to_vec(), QoS::AtMostOnce).unwrap();
        assert_eq!(store.count(), 2);

        store.clear();
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_total_payload_size() {
        let store = RetainStore::new();
        
        store.store("a", vec![0u8; 100], QoS::AtMostOnce).unwrap();
        store.store("b", vec![0u8; 200], QoS::AtMostOnce).unwrap();
        
        assert_eq!(store.total_payload_size(), 300);
    }
}

mod router_tests {
    use super::*;

    #[test]
    fn test_route_to_subscribers() {
        let router = MessageRouter::new();
        
        router.subscribe("client1", "sensor/+/temp", QoS::AtLeastOnce).unwrap();
        router.subscribe("client2", "sensor/#", QoS::ExactlyOnce).unwrap();

        let result = router.route("sensor/room1/temp", b"25".to_vec(), QoS::ExactlyOnce, false).unwrap();
        
        assert_eq!(result.subscriber_count, 2);
        assert_eq!(result.messages.len(), 2);
        assert!(!result.retained);
    }

    #[test]
    fn test_route_with_retain() {
        let router = MessageRouter::new();
        
        router.subscribe("client1", "test/topic", QoS::AtLeastOnce).unwrap();

        let result = router.route("test/topic", b"data".to_vec(), QoS::AtMostOnce, true).unwrap();
        
        assert!(result.retained);
        
        // Check retained message was stored
        let retained = router.retain_store().get("test/topic");
        assert!(retained.is_some());
    }

    #[test]
    fn test_subscribe_gets_retained() {
        let router = MessageRouter::new();
        
        // Store retained message first
        router.route("sensor/room1/temp", b"20".to_vec(), QoS::AtLeastOnce, true).unwrap();
        router.route("sensor/room1/humidity", b"50".to_vec(), QoS::AtLeastOnce, true).unwrap();

        // Subscribe should get retained messages
        let (qos, retained) = router.subscribe("client1", "sensor/+/temp", QoS::ExactlyOnce).unwrap();
        
        assert_eq!(qos, QoS::ExactlyOnce);
        assert_eq!(retained.len(), 1);
        assert!(retained[0].is_retained);
        assert_eq!(retained[0].topic, "sensor/room1/temp");
    }

    #[test]
    fn test_qos_downgrade() {
        let router = MessageRouter::new();
        
        // Subscribe with QoS 1
        router.subscribe("client1", "test/topic", QoS::AtLeastOnce).unwrap();

        // Publish with QoS 2
        let result = router.route("test/topic", b"data".to_vec(), QoS::ExactlyOnce, false).unwrap();
        
        // Message should be downgraded to QoS 1
        assert_eq!(result.messages[0].qos, QoS::AtLeastOnce);
    }

    #[test]
    fn test_route_excluding() {
        let router = MessageRouter::new();
        
        router.subscribe("client1", "test/topic", QoS::AtMostOnce).unwrap();
        router.subscribe("client2", "test/topic", QoS::AtMostOnce).unwrap();
        router.subscribe("client3", "test/topic", QoS::AtMostOnce).unwrap();

        // Route excluding client2
        let result = router.route_excluding(
            "test/topic",
            b"data".to_vec(),
            QoS::AtMostOnce,
            false,
            "client2"
        ).unwrap();
        
        assert_eq!(result.subscriber_count, 2);
        assert!(result.messages.iter().all(|m| m.client_id != "client2"));
    }

    #[test]
    fn test_remove_client_clears_subscriptions() {
        let router = MessageRouter::new();
        
        router.subscribe("client1", "a", QoS::AtMostOnce).unwrap();
        router.subscribe("client1", "b", QoS::AtMostOnce).unwrap();
        
        let removed = router.remove_client("client1");
        assert_eq!(removed.len(), 2);

        // Should no longer match
        let result = router.route("a", b"x".to_vec(), QoS::AtMostOnce, false).unwrap();
        assert_eq!(result.subscriber_count, 0);
    }

    #[test]
    fn test_stats() {
        let router = MessageRouter::new();
        
        router.subscribe("client1", "a", QoS::AtMostOnce).unwrap();
        router.subscribe("client2", "b", QoS::AtMostOnce).unwrap();
        router.route("c", b"retained".to_vec(), QoS::AtMostOnce, true).unwrap();

        let stats = router.stats();
        assert_eq!(stats.subscription_count, 2);
        assert_eq!(stats.client_count, 2);
        assert_eq!(stats.retained_message_count, 1);
        assert_eq!(stats.retained_payload_bytes, 8); // "retained".len()
    }

    #[test]
    fn test_empty_retain_deletes() {
        let router = MessageRouter::new();
        
        // Store retained
        router.route("test/topic", b"data".to_vec(), QoS::AtMostOnce, true).unwrap();
        assert_eq!(router.retain_store().count(), 1);

        // Delete with empty payload
        router.route("test/topic", vec![], QoS::AtMostOnce, true).unwrap();
        assert_eq!(router.retain_store().count(), 0);
    }
}
