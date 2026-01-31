//! Tests for the server module.

use super::*;
use crate::codec::QoS;
use crate::server::client::ClientStats;
use std::net::SocketAddr;

mod client_tests {
    use super::*;

    #[test]
    fn test_new_client() {
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let client = Client::new(addr);

        assert!(client.is_connecting());
        assert!(!client.is_connected());
        assert!(client.client_id.is_empty());
        assert_eq!(client.peer_addr, addr);
    }

    #[test]
    fn test_client_initialize() {
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut client = Client::new(addr);

        client.initialize("test-client".to_string(), true, 60, None);

        assert!(client.is_connected());
        assert_eq!(client.client_id, "test-client");
        assert!(client.clean_session);
        assert_eq!(client.keep_alive, 60);
    }

    #[test]
    fn test_client_auto_id_generation() {
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut client = Client::new(addr);

        client.initialize(String::new(), true, 0, None);

        assert!(client.client_id.starts_with("auto-"));
        assert_eq!(client.client_id.len(), 21); // "auto-" + 16 hex chars
    }

    #[test]
    fn test_client_keep_alive() {
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut client = Client::new(addr);
        client.initialize("test".to_string(), true, 1, None);

        // Just connected, should not be expired
        assert!(!client.is_keep_alive_expired());

        // With keep_alive=0, should never expire
        client.keep_alive = 0;
        assert!(!client.is_keep_alive_expired());
    }

    #[test]
    fn test_client_disconnect() {
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut client = Client::new(addr);
        client.initialize("test".to_string(), true, 0, None);

        assert!(client.is_connected());
        
        client.disconnect();
        assert!(!client.is_connected());
        assert!(matches!(client.state, ClientState::Disconnecting));

        client.mark_disconnected();
        assert!(matches!(client.state, ClientState::Disconnected));
    }

    #[test]
    fn test_client_subscriptions() {
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut client = Client::new(addr);
        client.initialize("test".to_string(), true, 0, None);

        client.subscribe("test/+/topic", QoS::AtLeastOnce);
        client.subscribe("sensor/#", QoS::ExactlyOnce);

        assert_eq!(client.subscriptions().len(), 2);

        let removed = client.unsubscribe("test/+/topic");
        assert!(removed);
        assert_eq!(client.subscriptions().len(), 1);

        let removed = client.unsubscribe("nonexistent");
        assert!(!removed);
    }

    #[test]
    fn test_client_stats() {
        let mut stats = ClientStats::default();

        stats.record_received(100);
        stats.record_sent(50);
        stats.record_publish();
        stats.record_delivery();

        assert_eq!(stats.packets_received, 1);
        assert_eq!(stats.packets_sent, 1);
        assert_eq!(stats.bytes_received, 100);
        assert_eq!(stats.bytes_sent, 50);
        assert_eq!(stats.messages_published, 1);
        assert_eq!(stats.messages_delivered, 1);
    }
}

mod config_tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();

        assert_eq!(config.bind_addr, "0.0.0.0:1883".parse().unwrap());
        assert_eq!(config.max_connections, 10000);
    }

    #[test]
    fn test_custom_config() {
        let addr: SocketAddr = "127.0.0.1:8883".parse().unwrap();
        let config = ServerConfig::new(addr);

        assert_eq!(config.bind_addr, addr);
    }
}

mod server_tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let config = ServerConfig::default();
        let server = Server::new(config);

        assert_eq!(server.client_count(), 0);
        assert_eq!(server.bind_addr(), "0.0.0.0:1883".parse().unwrap());
    }

    #[test]
    fn test_server_stats() {
        let server = Server::with_defaults();
        let stats = server.stats();

        assert_eq!(stats.connected_clients, 0);
        assert_eq!(stats.total_subscriptions, 0);
        assert_eq!(stats.retained_messages, 0);
    }
}
