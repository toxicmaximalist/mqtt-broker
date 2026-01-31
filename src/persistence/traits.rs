//! Persistence traits defining the storage interface.

use std::time::Instant;
use async_trait::async_trait;

use crate::codec::QoS;

/// Errors that can occur during persistence operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PersistenceError {
    #[error("Session not found: {client_id}")]
    SessionNotFound { client_id: String },

    #[error("Message not found: packet_id={packet_id}")]
    MessageNotFound { packet_id: u16 },

    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Storage full")]
    StorageFull,
}

/// Persisted session state for clean_session=false reconnects.
#[derive(Debug, Clone)]
pub struct PersistedSession {
    /// The client ID.
    pub client_id: String,
    /// Topic filters the client is subscribed to with their QoS.
    pub subscriptions: Vec<(String, QoS)>,
    /// When this session was created.
    pub created_at: Instant,
    /// When this session was last active.
    pub last_active: Instant,
    /// Session expiry interval in seconds (0 = never expire while broker is running).
    pub expiry_interval: u32,
}

impl PersistedSession {
    /// Create a new persisted session.
    pub fn new(client_id: impl Into<String>) -> Self {
        let now = Instant::now();
        Self {
            client_id: client_id.into(),
            subscriptions: Vec::new(),
            created_at: now,
            last_active: now,
            expiry_interval: 0,
        }
    }

    /// Check if this session has expired.
    pub fn is_expired(&self) -> bool {
        if self.expiry_interval == 0 {
            return false;
        }
        self.last_active.elapsed().as_secs() > self.expiry_interval as u64
    }

    /// Update the last active time to now.
    pub fn touch(&mut self) {
        self.last_active = Instant::now();
    }
}

/// A pending message waiting for acknowledgment.
#[derive(Debug, Clone)]
pub struct PendingMessage {
    /// The packet ID for this message.
    pub packet_id: u16,
    /// The topic this message was published to.
    pub topic: String,
    /// The message payload.
    pub payload: Vec<u8>,
    /// The QoS level.
    pub qos: QoS,
    /// Whether this is a retained message.
    pub retain: bool,
    /// When this message was stored.
    pub stored_at: Instant,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
    /// Current state for QoS 2 flows.
    pub qos2_state: Option<Qos2State>,
}

/// QoS 2 delivery state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qos2State {
    /// PUBLISH sent, waiting for PUBREC.
    WaitingPubrec,
    /// PUBREC received, PUBREL sent, waiting for PUBCOMP.
    WaitingPubcomp,
}

impl PendingMessage {
    /// Create a new pending message.
    pub fn new(
        packet_id: u16,
        topic: impl Into<String>,
        payload: Vec<u8>,
        qos: QoS,
        retain: bool,
    ) -> Self {
        Self {
            packet_id,
            topic: topic.into(),
            payload,
            qos,
            retain,
            stored_at: Instant::now(),
            delivery_attempts: 0,
            qos2_state: match qos {
                QoS::ExactlyOnce => Some(Qos2State::WaitingPubrec),
                _ => None,
            },
        }
    }

    /// Get the age of this pending message.
    pub fn age(&self) -> std::time::Duration {
        self.stored_at.elapsed()
    }
}

/// Trait for session state persistence.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Save or update a session.
    async fn save_session(&self, session: PersistedSession) -> Result<(), PersistenceError>;

    /// Load a session by client ID.
    async fn load_session(&self, client_id: &str) -> Result<Option<PersistedSession>, PersistenceError>;

    /// Delete a session.
    async fn delete_session(&self, client_id: &str) -> Result<bool, PersistenceError>;

    /// Check if a session exists.
    async fn session_exists(&self, client_id: &str) -> Result<bool, PersistenceError>;

    /// Get all session client IDs.
    async fn list_sessions(&self) -> Result<Vec<String>, PersistenceError>;

    /// Remove expired sessions.
    async fn cleanup_expired(&self) -> Result<usize, PersistenceError>;
}

/// Trait for pending message persistence.
#[async_trait]
pub trait MessageStore: Send + Sync {
    /// Store a pending message for a client.
    async fn store_pending(
        &self,
        client_id: &str,
        message: PendingMessage,
    ) -> Result<(), PersistenceError>;

    /// Get a pending message by packet ID.
    async fn get_pending(
        &self,
        client_id: &str,
        packet_id: u16,
    ) -> Result<Option<PendingMessage>, PersistenceError>;

    /// Update a pending message (e.g., increment delivery attempts).
    async fn update_pending(
        &self,
        client_id: &str,
        message: PendingMessage,
    ) -> Result<(), PersistenceError>;

    /// Remove a pending message (after successful delivery).
    async fn remove_pending(
        &self,
        client_id: &str,
        packet_id: u16,
    ) -> Result<bool, PersistenceError>;

    /// Get all pending messages for a client (for resend on reconnect).
    async fn get_all_pending(
        &self,
        client_id: &str,
    ) -> Result<Vec<PendingMessage>, PersistenceError>;

    /// Clear all pending messages for a client.
    async fn clear_pending(&self, client_id: &str) -> Result<usize, PersistenceError>;

    /// Get count of pending messages for a client.
    async fn pending_count(&self, client_id: &str) -> Result<usize, PersistenceError>;
}
