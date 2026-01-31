//! Retained message store.
//!
//! Stores the most recent retained message for each topic.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

use crate::codec::QoS;

/// A retained message stored in the broker.
#[derive(Debug, Clone)]
pub struct RetainedMessage {
    /// The topic this message is retained on.
    pub topic: String,
    /// The message payload (empty payload means delete).
    pub payload: Vec<u8>,
    /// The QoS level of the retained message.
    pub qos: QoS,
    /// When this message was retained.
    pub timestamp: Instant,
}

impl RetainedMessage {
    /// Create a new retained message.
    pub fn new(topic: impl Into<String>, payload: Vec<u8>, qos: QoS) -> Self {
        Self {
            topic: topic.into(),
            payload,
            qos,
            timestamp: Instant::now(),
        }
    }

    /// Check if this is a delete marker (empty payload).
    pub fn is_delete(&self) -> bool {
        self.payload.is_empty()
    }

    /// Get the age of this retained message.
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Store for retained messages.
///
/// Per MQTT spec, only one retained message is kept per topic.
/// Sending a retained message with empty payload deletes the retained message.
pub struct RetainStore {
    /// Topic -> retained message mapping.
    messages: RwLock<HashMap<String, RetainedMessage>>,
    /// Maximum number of retained messages (0 = unlimited).
    max_messages: usize,
    /// Maximum payload size for retained messages.
    max_payload_size: usize,
}

impl RetainStore {
    /// Create a new retain store with default limits.
    pub fn new() -> Self {
        Self {
            messages: RwLock::new(HashMap::new()),
            max_messages: 10000,        // Default: 10k retained messages
            max_payload_size: 256 * 1024, // Default: 256KB payload
        }
    }

    /// Create a new retain store with custom limits.
    pub fn with_limits(max_messages: usize, max_payload_size: usize) -> Self {
        Self {
            messages: RwLock::new(HashMap::new()),
            max_messages,
            max_payload_size,
        }
    }

    /// Store or update a retained message.
    ///
    /// If payload is empty, the retained message is deleted.
    /// Returns true if a new message was stored, false if updated or deleted.
    pub fn store(&self, topic: &str, payload: Vec<u8>, qos: QoS) -> Result<bool, RetainError> {
        // Empty payload = delete
        if payload.is_empty() {
            let mut messages = self.messages.write();
            return Ok(messages.remove(topic).is_some());
        }

        // Check payload size
        if self.max_payload_size > 0 && payload.len() > self.max_payload_size {
            return Err(RetainError::PayloadTooLarge {
                size: payload.len(),
                max: self.max_payload_size,
            });
        }

        let mut messages = self.messages.write();

        // Check message count limit
        if self.max_messages > 0 && messages.len() >= self.max_messages && !messages.contains_key(topic) {
            return Err(RetainError::StoreFull {
                count: messages.len(),
                max: self.max_messages,
            });
        }

        let is_new = !messages.contains_key(topic);
        messages.insert(topic.to_string(), RetainedMessage::new(topic, payload, qos));
        Ok(is_new)
    }

    /// Get a retained message for a specific topic.
    pub fn get(&self, topic: &str) -> Option<RetainedMessage> {
        self.messages.read().get(topic).cloned()
    }

    /// Get all retained messages matching a topic filter.
    ///
    /// Uses the same wildcard matching rules as subscriptions.
    pub fn get_matching(&self, filter: &str) -> Vec<RetainedMessage> {
        use crate::topic_matcher::matches_filter;

        let messages = self.messages.read();
        messages
            .values()
            .filter(|msg| matches_filter(&msg.topic, filter))
            .cloned()
            .collect()
    }

    /// Remove a retained message.
    pub fn remove(&self, topic: &str) -> Option<RetainedMessage> {
        self.messages.write().remove(topic)
    }

    /// Clear all retained messages.
    pub fn clear(&self) {
        self.messages.write().clear();
    }

    /// Get the number of retained messages.
    pub fn count(&self) -> usize {
        self.messages.read().len()
    }

    /// Get total size of all retained payloads.
    pub fn total_payload_size(&self) -> usize {
        self.messages.read().values().map(|m| m.payload.len()).sum()
    }

    /// Get all topic names with retained messages.
    pub fn topics(&self) -> Vec<String> {
        self.messages.read().keys().cloned().collect()
    }

    /// Remove retained messages older than the given duration.
    pub fn expire_older_than(&self, max_age: Duration) -> usize {
        let mut messages = self.messages.write();
        let before = messages.len();
        messages.retain(|_, msg| msg.age() < max_age);
        before - messages.len()
    }
}

impl Default for RetainStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when storing retained messages.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RetainError {
    #[error("Payload too large: {size} bytes (max: {max})")]
    PayloadTooLarge { size: usize, max: usize },

    #[error("Retain store full: {count} messages (max: {max})")]
    StoreFull { count: usize, max: usize },
}

// Thread-safe markers
unsafe impl Send for RetainStore {}
unsafe impl Sync for RetainStore {}
