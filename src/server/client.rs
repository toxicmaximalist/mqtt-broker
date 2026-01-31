//! Client representation and state management.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::codec::QoS;
use crate::session::{Session, SessionConfig};

/// Unique client identifier type.
pub type ClientId = String;

/// Atomic counter for generating unique internal client IDs.
static CLIENT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique internal client ID for anonymous clients.
fn generate_client_id() -> ClientId {
    let id = CLIENT_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("auto-{:016x}", id)
}

/// Client connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Waiting for CONNECT packet.
    Connecting,
    /// Connected and operational.
    Connected,
    /// Disconnecting (DISCONNECT received or being sent).
    Disconnecting,
    /// Connection closed.
    Disconnected,
}

/// Statistics for a client connection.
#[derive(Debug, Clone, Default)]
pub struct ClientStats {
    /// Total packets received from this client.
    pub packets_received: u64,
    /// Total packets sent to this client.
    pub packets_sent: u64,
    /// Total bytes received.
    pub bytes_received: u64,
    /// Total bytes sent.
    pub bytes_sent: u64,
    /// Messages published by this client.
    pub messages_published: u64,
    /// Messages delivered to this client.
    pub messages_delivered: u64,
}

impl ClientStats {
    /// Record a received packet.
    pub fn record_received(&mut self, bytes: usize) {
        self.packets_received += 1;
        self.bytes_received += bytes as u64;
    }

    /// Record a sent packet.
    pub fn record_sent(&mut self, bytes: usize) {
        self.packets_sent += 1;
        self.bytes_sent += bytes as u64;
    }

    /// Record a publish from this client.
    pub fn record_publish(&mut self) {
        self.messages_published += 1;
    }

    /// Record a message delivered to this client.
    pub fn record_delivery(&mut self) {
        self.messages_delivered += 1;
    }
}

/// A connected MQTT client.
#[derive(Debug)]
pub struct Client {
    /// The MQTT client ID.
    pub client_id: ClientId,
    /// Remote peer address.
    pub peer_addr: SocketAddr,
    /// Current connection state.
    pub state: ClientState,
    /// Session state.
    pub session: Session,
    /// Connection statistics.
    pub stats: ClientStats,
    /// When this client connected.
    pub connected_at: Option<Instant>,
    /// Keep-alive interval in seconds (0 = disabled).
    pub keep_alive: u16,
    /// Whether this is a clean session.
    pub clean_session: bool,
    /// Last activity time (for keep-alive).
    pub last_activity: Instant,
    /// Username (if authenticated).
    pub username: Option<String>,
}

impl Client {
    /// Create a new client in connecting state.
    pub fn new(peer_addr: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            client_id: String::new(), // Set on CONNECT
            peer_addr,
            state: ClientState::Connecting,
            session: Session::new("".to_string(), true, SessionConfig::default()),
            stats: ClientStats::default(),
            connected_at: None,
            keep_alive: 0,
            clean_session: true,
            last_activity: now,
            username: None,
        }
    }

    /// Initialize the client from a CONNECT packet.
    pub fn initialize(
        &mut self,
        client_id: String,
        clean_session: bool,
        keep_alive: u16,
        username: Option<String>,
    ) {
        let final_client_id = if client_id.is_empty() {
            generate_client_id()
        } else {
            client_id
        };

        self.client_id = final_client_id.clone();
        self.clean_session = clean_session;
        self.keep_alive = keep_alive;
        self.username = username;
        self.connected_at = Some(Instant::now());
        self.state = ClientState::Connected;
        self.session = Session::new(final_client_id, clean_session, SessionConfig::default());
    }

    /// Restore from an existing session.
    pub fn restore_session(&mut self, session: Session) {
        self.session = session;
    }

    /// Update last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if keep-alive has expired.
    pub fn is_keep_alive_expired(&self) -> bool {
        if self.keep_alive == 0 {
            return false;
        }
        // Allow 1.5x keep-alive interval per MQTT spec
        let timeout = (self.keep_alive as f32 * 1.5) as u64;
        self.last_activity.elapsed().as_secs() > timeout
    }

    /// Get the connection duration.
    pub fn connection_duration(&self) -> Option<std::time::Duration> {
        self.connected_at.map(|t| t.elapsed())
    }

    /// Mark the client as disconnecting.
    pub fn disconnect(&mut self) {
        self.state = ClientState::Disconnecting;
    }

    /// Mark the client as fully disconnected.
    pub fn mark_disconnected(&mut self) {
        self.state = ClientState::Disconnected;
    }

    /// Check if the client is still connected.
    pub fn is_connected(&self) -> bool {
        matches!(self.state, ClientState::Connected)
    }

    /// Check if the client is in connecting state.
    pub fn is_connecting(&self) -> bool {
        matches!(self.state, ClientState::Connecting)
    }

    /// Subscribe to a topic filter.
    pub fn subscribe(&mut self, filter: &str, qos: QoS) {
        self.session.subscribe(filter.to_string(), qos);
    }

    /// Unsubscribe from a topic filter.
    pub fn unsubscribe(&mut self, filter: &str) -> bool {
        self.session.unsubscribe(filter)
    }

    /// Get the client's subscriptions.
    pub fn subscriptions(&self) -> &HashMap<String, QoS> {
        self.session.subscriptions()
    }
}
