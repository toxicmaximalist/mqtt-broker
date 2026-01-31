//! Tests for the persistence module.

use super::*;
use crate::codec::QoS;

mod session_store_tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_session() {
        let store = MemoryStore::new();

        let session = PersistedSession::new("client1");
        store.save_session(session.clone()).await.unwrap();

        let loaded = store.load_session("client1").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().client_id, "client1");
    }

    #[tokio::test]
    async fn test_load_nonexistent_session() {
        let store = MemoryStore::new();
        let loaded = store.load_session("nonexistent").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = MemoryStore::new();

        let session = PersistedSession::new("client1");
        store.save_session(session).await.unwrap();

        assert!(store.session_exists("client1").await.unwrap());

        let deleted = store.delete_session("client1").await.unwrap();
        assert!(deleted);

        assert!(!store.session_exists("client1").await.unwrap());

        // Delete again should return false
        let deleted = store.delete_session("client1").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_update_session() {
        let store = MemoryStore::new();

        let mut session = PersistedSession::new("client1");
        store.save_session(session.clone()).await.unwrap();

        // Update with subscriptions
        session.subscriptions.push(("test/topic".to_string(), QoS::AtLeastOnce));
        store.save_session(session).await.unwrap();

        let loaded = store.load_session("client1").await.unwrap().unwrap();
        assert_eq!(loaded.subscriptions.len(), 1);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let store = MemoryStore::new();

        store.save_session(PersistedSession::new("client1")).await.unwrap();
        store.save_session(PersistedSession::new("client2")).await.unwrap();
        store.save_session(PersistedSession::new("client3")).await.unwrap();

        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_session_capacity() {
        let store = MemoryStore::with_limits(2, 100);

        store.save_session(PersistedSession::new("client1")).await.unwrap();
        store.save_session(PersistedSession::new("client2")).await.unwrap();

        // Third should fail
        let result = store.save_session(PersistedSession::new("client3")).await;
        assert!(matches!(result, Err(PersistenceError::StorageFull)));

        // But updating existing should work
        store.save_session(PersistedSession::new("client1")).await.unwrap();
    }

    #[tokio::test]
    async fn test_session_expiry() {
        let store = MemoryStore::new();

        let mut session = PersistedSession::new("client1");
        session.expiry_interval = 0; // Never expires
        assert!(!session.is_expired());

        // With expiry set, check logic
        let mut session2 = PersistedSession::new("client2");
        session2.expiry_interval = 3600; // 1 hour
        assert!(!session2.is_expired()); // Just created
    }
}

mod message_store_tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_get_pending() {
        let store = MemoryStore::new();

        let msg = PendingMessage::new(
            1,
            "test/topic",
            b"hello".to_vec(),
            QoS::AtLeastOnce,
            false,
        );
        store.store_pending("client1", msg).await.unwrap();

        let loaded = store.get_pending("client1", 1).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.packet_id, 1);
        assert_eq!(loaded.topic, "test/topic");
        assert_eq!(loaded.payload, b"hello");
    }

    #[tokio::test]
    async fn test_get_nonexistent_pending() {
        let store = MemoryStore::new();

        let loaded = store.get_pending("client1", 1).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_remove_pending() {
        let store = MemoryStore::new();

        let msg = PendingMessage::new(1, "topic", vec![], QoS::AtLeastOnce, false);
        store.store_pending("client1", msg).await.unwrap();

        let removed = store.remove_pending("client1", 1).await.unwrap();
        assert!(removed);

        let loaded = store.get_pending("client1", 1).await.unwrap();
        assert!(loaded.is_none());

        // Remove again should return false
        let removed = store.remove_pending("client1", 1).await.unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_update_pending() {
        let store = MemoryStore::new();

        let mut msg = PendingMessage::new(1, "topic", vec![], QoS::AtLeastOnce, false);
        store.store_pending("client1", msg.clone()).await.unwrap();

        // Update delivery attempts
        msg.delivery_attempts = 5;
        store.update_pending("client1", msg).await.unwrap();

        let loaded = store.get_pending("client1", 1).await.unwrap().unwrap();
        assert_eq!(loaded.delivery_attempts, 5);
    }

    #[tokio::test]
    async fn test_update_nonexistent_pending() {
        let store = MemoryStore::new();

        let msg = PendingMessage::new(1, "topic", vec![], QoS::AtLeastOnce, false);

        // Should fail - no session
        let result = store.update_pending("client1", msg.clone()).await;
        assert!(matches!(result, Err(PersistenceError::SessionNotFound { .. })));

        // Add a different message, then try to update nonexistent
        let msg2 = PendingMessage::new(2, "topic", vec![], QoS::AtLeastOnce, false);
        store.store_pending("client1", msg2).await.unwrap();

        let result = store.update_pending("client1", msg).await;
        assert!(matches!(result, Err(PersistenceError::MessageNotFound { .. })));
    }

    #[tokio::test]
    async fn test_get_all_pending() {
        let store = MemoryStore::new();

        for i in 1..=5 {
            let msg = PendingMessage::new(i, format!("topic/{}", i), vec![], QoS::AtLeastOnce, false);
            store.store_pending("client1", msg).await.unwrap();
        }

        let all = store.get_all_pending("client1").await.unwrap();
        assert_eq!(all.len(), 5);
    }

    #[tokio::test]
    async fn test_clear_pending() {
        let store = MemoryStore::new();

        for i in 1..=5 {
            let msg = PendingMessage::new(i, "topic", vec![], QoS::AtLeastOnce, false);
            store.store_pending("client1", msg).await.unwrap();
        }

        let cleared = store.clear_pending("client1").await.unwrap();
        assert_eq!(cleared, 5);

        let count = store.pending_count("client1").await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_pending_capacity() {
        let store = MemoryStore::with_limits(100, 3);

        for i in 1..=3 {
            let msg = PendingMessage::new(i, "topic", vec![], QoS::AtLeastOnce, false);
            store.store_pending("client1", msg).await.unwrap();
        }

        // Fourth should fail
        let msg = PendingMessage::new(4, "topic", vec![], QoS::AtLeastOnce, false);
        let result = store.store_pending("client1", msg).await;
        assert!(matches!(result, Err(PersistenceError::StorageFull)));
    }

    #[tokio::test]
    async fn test_pending_count() {
        let store = MemoryStore::new();

        assert_eq!(store.pending_count("client1").await.unwrap(), 0);

        for i in 1..=3 {
            let msg = PendingMessage::new(i, "topic", vec![], QoS::AtLeastOnce, false);
            store.store_pending("client1", msg).await.unwrap();
        }

        assert_eq!(store.pending_count("client1").await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_qos2_state() {
        // QoS 2 messages should start with WaitingPubrec state
        let msg = PendingMessage::new(1, "topic", vec![], QoS::ExactlyOnce, false);
        assert_eq!(msg.qos2_state, Some(Qos2State::WaitingPubrec));

        // QoS 1 messages should have no QoS 2 state
        let msg = PendingMessage::new(2, "topic", vec![], QoS::AtLeastOnce, false);
        assert_eq!(msg.qos2_state, None);
    }

    #[tokio::test]
    async fn test_delete_session_clears_pending() {
        let store = MemoryStore::new();

        store.save_session(PersistedSession::new("client1")).await.unwrap();
        
        for i in 1..=3 {
            let msg = PendingMessage::new(i, "topic", vec![], QoS::AtLeastOnce, false);
            store.store_pending("client1", msg).await.unwrap();
        }

        // Delete session should also clear pending
        store.delete_session("client1").await.unwrap();

        assert_eq!(store.pending_count("client1").await.unwrap(), 0);
    }
}

mod memory_store_tests {
    use super::*;

    #[test]
    fn test_default() {
        let store = MemoryStore::default();
        assert_eq!(store.session_count(), 0);
        assert_eq!(store.total_pending_count(), 0);
    }

    #[tokio::test]
    async fn test_clear() {
        let store = MemoryStore::new();

        store.save_session(PersistedSession::new("client1")).await.unwrap();
        let msg = PendingMessage::new(1, "topic", vec![], QoS::AtLeastOnce, false);
        store.store_pending("client1", msg).await.unwrap();

        assert_eq!(store.session_count(), 1);
        assert_eq!(store.total_pending_count(), 1);

        store.clear();

        assert_eq!(store.session_count(), 0);
        assert_eq!(store.total_pending_count(), 0);
    }
}
