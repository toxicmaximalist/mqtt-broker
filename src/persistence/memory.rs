//! In-memory persistence implementation.
//!
//! This is the default storage backend, suitable for development and
//! brokers that don't need persistence across restarts.

use std::collections::HashMap;
use async_trait::async_trait;
use parking_lot::RwLock;

use super::traits::{
    MessageStore, PendingMessage, PersistenceError, PersistedSession, SessionStore,
};

/// In-memory storage for sessions and messages.
///
/// All data is lost on broker restart. For persistent sessions across restarts,
/// use a file-based or database-backed store.
pub struct MemoryStore {
    /// Session storage.
    sessions: RwLock<HashMap<String, PersistedSession>>,
    /// Pending messages per client.
    pending: RwLock<HashMap<String, HashMap<u16, PendingMessage>>>,
    /// Maximum sessions to store.
    max_sessions: usize,
    /// Maximum pending messages per client.
    max_pending_per_client: usize,
}

impl MemoryStore {
    /// Create a new memory store with default limits.
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashMap::new()),
            max_sessions: 100_000,
            max_pending_per_client: 1000,
        }
    }

    /// Create a new memory store with custom limits.
    pub fn with_limits(max_sessions: usize, max_pending_per_client: usize) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashMap::new()),
            max_sessions,
            max_pending_per_client,
        }
    }

    /// Get the number of stored sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.read().len()
    }

    /// Get total pending message count across all clients.
    pub fn total_pending_count(&self) -> usize {
        self.pending.read().values().map(|m| m.len()).sum()
    }

    /// Clear all data.
    pub fn clear(&self) {
        self.sessions.write().clear();
        self.pending.write().clear();
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn save_session(&self, session: PersistedSession) -> Result<(), PersistenceError> {
        let mut sessions = self.sessions.write();

        // Check capacity for new sessions
        if !sessions.contains_key(&session.client_id) && sessions.len() >= self.max_sessions {
            return Err(PersistenceError::StorageFull);
        }

        sessions.insert(session.client_id.clone(), session);
        Ok(())
    }

    async fn load_session(&self, client_id: &str) -> Result<Option<PersistedSession>, PersistenceError> {
        Ok(self.sessions.read().get(client_id).cloned())
    }

    async fn delete_session(&self, client_id: &str) -> Result<bool, PersistenceError> {
        let removed = self.sessions.write().remove(client_id).is_some();
        // Also clear pending messages
        self.pending.write().remove(client_id);
        Ok(removed)
    }

    async fn session_exists(&self, client_id: &str) -> Result<bool, PersistenceError> {
        Ok(self.sessions.read().contains_key(client_id))
    }

    async fn list_sessions(&self) -> Result<Vec<String>, PersistenceError> {
        Ok(self.sessions.read().keys().cloned().collect())
    }

    async fn cleanup_expired(&self) -> Result<usize, PersistenceError> {
        let mut sessions = self.sessions.write();
        let before = sessions.len();
        sessions.retain(|_, session| !session.is_expired());
        let removed = before - sessions.len();

        // Also cleanup pending for removed sessions
        let mut pending = self.pending.write();
        let session_ids: std::collections::HashSet<_> = sessions.keys().cloned().collect();
        pending.retain(|client_id, _| session_ids.contains(client_id));

        Ok(removed)
    }
}

#[async_trait]
impl MessageStore for MemoryStore {
    async fn store_pending(
        &self,
        client_id: &str,
        message: PendingMessage,
    ) -> Result<(), PersistenceError> {
        let mut pending = self.pending.write();
        let client_pending = pending.entry(client_id.to_string()).or_default();

        // Check capacity
        if client_pending.len() >= self.max_pending_per_client {
            return Err(PersistenceError::StorageFull);
        }

        client_pending.insert(message.packet_id, message);
        Ok(())
    }

    async fn get_pending(
        &self,
        client_id: &str,
        packet_id: u16,
    ) -> Result<Option<PendingMessage>, PersistenceError> {
        Ok(self
            .pending
            .read()
            .get(client_id)
            .and_then(|m| m.get(&packet_id))
            .cloned())
    }

    async fn update_pending(
        &self,
        client_id: &str,
        message: PendingMessage,
    ) -> Result<(), PersistenceError> {
        let mut pending = self.pending.write();
        let client_pending = pending
            .get_mut(client_id)
            .ok_or_else(|| PersistenceError::SessionNotFound {
                client_id: client_id.to_string(),
            })?;

        if !client_pending.contains_key(&message.packet_id) {
            return Err(PersistenceError::MessageNotFound {
                packet_id: message.packet_id,
            });
        }

        client_pending.insert(message.packet_id, message);
        Ok(())
    }

    async fn remove_pending(
        &self,
        client_id: &str,
        packet_id: u16,
    ) -> Result<bool, PersistenceError> {
        let mut pending = self.pending.write();
        if let Some(client_pending) = pending.get_mut(client_id) {
            Ok(client_pending.remove(&packet_id).is_some())
        } else {
            Ok(false)
        }
    }

    async fn get_all_pending(
        &self,
        client_id: &str,
    ) -> Result<Vec<PendingMessage>, PersistenceError> {
        Ok(self
            .pending
            .read()
            .get(client_id)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn clear_pending(&self, client_id: &str) -> Result<usize, PersistenceError> {
        let mut pending = self.pending.write();
        if let Some(client_pending) = pending.remove(client_id) {
            Ok(client_pending.len())
        } else {
            Ok(0)
        }
    }

    async fn pending_count(&self, client_id: &str) -> Result<usize, PersistenceError> {
        Ok(self
            .pending
            .read()
            .get(client_id)
            .map(|m| m.len())
            .unwrap_or(0))
    }
}
