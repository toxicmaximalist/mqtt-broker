//! MQTT Session state

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use bytes::Bytes;

use crate::codec::QoS;

use super::packet_id::PacketIdAllocator;
use super::qos::{PendingPublish, QoS1State, QoS2InboundState, QoS2OutboundState, QoSConfig, QoSError};

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum messages to queue for offline client
    pub max_offline_messages: usize,
    /// Maximum in-flight QoS 1/2 messages
    pub max_inflight: usize,
    /// Session expiry interval (None = infinite for persistent sessions)
    pub session_expiry: Option<Duration>,
    /// QoS retry configuration
    pub qos_config: QoSConfig,
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            max_offline_messages: 1000,
            max_inflight: 65535,
            session_expiry: Some(Duration::from_secs(24 * 60 * 60)), // 24 hours
            qos_config: QoSConfig::default(),
        }
    }
}

/// A queued message for offline delivery
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    pub topic: String,
    pub payload: Bytes,
    pub qos: QoS,
    pub retain: bool,
    pub queued_at: Instant,
}

/// MQTT Session - persists across connections when clean_session=false
#[derive(Debug)]
pub struct Session {
    /// Client identifier
    client_id: String,
    /// Whether this is a clean session
    clean_session: bool,
    /// Topic filter subscriptions with QoS
    subscriptions: HashMap<String, QoS>,
    /// Packet ID allocator for outbound messages
    packet_id_allocator: PacketIdAllocator,
    /// QoS 1 outbound messages awaiting PUBACK
    qos1_outbound: HashMap<u16, QoS1State>,
    /// QoS 2 outbound messages (we sent PUBLISH)
    qos2_outbound: HashMap<u16, QoS2OutboundState>,
    /// QoS 2 inbound messages (we received PUBLISH)
    qos2_inbound: HashMap<u16, QoS2InboundState>,
    /// Messages queued while client is offline
    offline_queue: VecDeque<QueuedMessage>,
    /// Session creation time
    created_at: Instant,
    /// Last activity time
    last_active: Instant,
    /// Configuration
    config: SessionConfig,
}

impl Session {
    /// Create a new session
    pub fn new(client_id: String, clean_session: bool, config: SessionConfig) -> Self {
        let now = Instant::now();
        Session {
            client_id,
            clean_session,
            subscriptions: HashMap::new(),
            packet_id_allocator: PacketIdAllocator::new(),
            qos1_outbound: HashMap::new(),
            qos2_outbound: HashMap::new(),
            qos2_inbound: HashMap::new(),
            offline_queue: VecDeque::new(),
            created_at: now,
            last_active: now,
            config,
        }
    }

    /// Get client ID
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Check if clean session
    pub fn is_clean_session(&self) -> bool {
        self.clean_session
    }

    /// Update last activity time
    pub fn touch(&mut self) {
        self.last_active = Instant::now();
    }

    /// Check if session has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.config.session_expiry {
            self.last_active.elapsed() > expiry
        } else {
            false
        }
    }

    // ========================================================================
    // Subscription management
    // ========================================================================

    /// Add or update a subscription
    pub fn subscribe(&mut self, topic_filter: String, qos: QoS) -> Option<QoS> {
        self.subscriptions.insert(topic_filter, qos)
    }

    /// Remove a subscription
    pub fn unsubscribe(&mut self, topic_filter: &str) -> bool {
        self.subscriptions.remove(topic_filter).is_some()
    }

    /// Get all subscriptions
    pub fn subscriptions(&self) -> &HashMap<String, QoS> {
        &self.subscriptions
    }

    /// Check if subscribed to a topic filter
    pub fn is_subscribed(&self, topic_filter: &str) -> bool {
        self.subscriptions.contains_key(topic_filter)
    }

    /// Clear all subscriptions
    pub fn clear_subscriptions(&mut self) {
        self.subscriptions.clear();
    }

    // ========================================================================
    // Packet ID management
    // ========================================================================

    /// Allocate a new packet ID for outbound QoS 1/2 message
    pub fn allocate_packet_id(&mut self) -> Option<u16> {
        self.packet_id_allocator.allocate()
    }

    /// Release a packet ID
    pub fn release_packet_id(&mut self, id: u16) {
        self.packet_id_allocator.release(id);
    }

    // ========================================================================
    // QoS 1 message handling
    // ========================================================================

    /// Start QoS 1 outbound delivery
    pub fn start_qos1_publish(&mut self, pending: PendingPublish) -> Result<(), QoSError> {
        let packet_id = pending.packet_id;
        if self.qos1_outbound.contains_key(&packet_id) {
            return Err(QoSError::DuplicatePacketId(packet_id));
        }
        self.qos1_outbound.insert(packet_id, QoS1State::new(pending));
        Ok(())
    }

    /// Handle PUBACK received
    pub fn on_puback(&mut self, packet_id: u16) -> Result<(), QoSError> {
        if let Some(state) = self.qos1_outbound.remove(&packet_id) {
            let new_state = state.on_puback(packet_id)?;
            if !new_state.is_complete() {
                self.qos1_outbound.insert(packet_id, new_state);
            } else {
                self.packet_id_allocator.release(packet_id);
            }
            Ok(())
        } else {
            // Unknown packet ID - might be duplicate ack, ignore
            Ok(())
        }
    }

    /// Get QoS 1 messages that need retransmission
    pub fn get_qos1_retries(&self) -> Vec<u16> {
        self.qos1_outbound
            .iter()
            .filter(|(_, state)| {
                state.pending()
                    .map(|p| p.should_retry(&self.config.qos_config))
                    .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Mark QoS 1 message as retried
    pub fn mark_qos1_retried(&mut self, packet_id: u16) {
        if let Some(state) = self.qos1_outbound.get_mut(&packet_id) {
            if let Some(pending) = state.pending_mut() {
                pending.mark_retried();
            }
        }
    }

    /// Get pending QoS 1 publish for retransmission
    pub fn get_qos1_pending(&self, packet_id: u16) -> Option<&PendingPublish> {
        self.qos1_outbound.get(&packet_id).and_then(|s| s.pending())
    }

    // ========================================================================
    // QoS 2 outbound message handling (we are sending)
    // ========================================================================

    /// Start QoS 2 outbound delivery
    pub fn start_qos2_publish(&mut self, pending: PendingPublish) -> Result<(), QoSError> {
        let packet_id = pending.packet_id;
        if self.qos2_outbound.contains_key(&packet_id) {
            return Err(QoSError::DuplicatePacketId(packet_id));
        }
        self.qos2_outbound.insert(packet_id, QoS2OutboundState::new(pending));
        Ok(())
    }

    /// Handle PUBREC received (QoS 2 step 2)
    pub fn on_pubrec(&mut self, packet_id: u16) -> Result<bool, QoSError> {
        if let Some(state) = self.qos2_outbound.remove(&packet_id) {
            let new_state = state.on_pubrec(packet_id)?;
            let needs_pubrel = matches!(new_state, QoS2OutboundState::AwaitingPubComp { .. });
            self.qos2_outbound.insert(packet_id, new_state);
            Ok(needs_pubrel)
        } else {
            // Unknown packet ID
            Ok(false)
        }
    }

    /// Handle PUBCOMP received (QoS 2 step 4)
    pub fn on_pubcomp(&mut self, packet_id: u16) -> Result<(), QoSError> {
        if let Some(state) = self.qos2_outbound.remove(&packet_id) {
            let new_state = state.on_pubcomp(packet_id)?;
            if !new_state.is_complete() {
                self.qos2_outbound.insert(packet_id, new_state);
            } else {
                self.packet_id_allocator.release(packet_id);
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Get QoS 2 outbound messages needing PUBLISH retry
    pub fn get_qos2_publish_retries(&self) -> Vec<u16> {
        self.qos2_outbound
            .iter()
            .filter_map(|(id, state)| {
                if let QoS2OutboundState::AwaitingPubRec(p) = state {
                    if p.should_retry(&self.config.qos_config) {
                        return Some(*id);
                    }
                }
                None
            })
            .collect()
    }

    /// Get QoS 2 outbound messages needing PUBREL retry
    pub fn get_qos2_pubrel_retries(&self) -> Vec<u16> {
        self.qos2_outbound
            .iter()
            .filter(|(_, state)| state.should_retry_pubrel(&self.config.qos_config))
            .map(|(id, _)| *id)
            .collect()
    }

    // ========================================================================
    // QoS 2 inbound message handling (we are receiving)
    // ========================================================================

    /// Start QoS 2 inbound flow (received PUBLISH, need to send PUBREC)
    /// Returns false if this is a duplicate (packet_id already tracked)
    pub fn start_qos2_receive(&mut self, packet_id: u16) -> bool {
        if self.qos2_inbound.contains_key(&packet_id) {
            // Duplicate PUBLISH - resend PUBREC
            return false;
        }
        self.qos2_inbound.insert(packet_id, QoS2InboundState::new(packet_id));
        true
    }

    /// Handle PUBREL received (QoS 2 step 3)
    pub fn on_pubrel(&mut self, packet_id: u16) -> Result<bool, QoSError> {
        if let Some(state) = self.qos2_inbound.remove(&packet_id) {
            let new_state = state.on_pubrel(packet_id)?;
            if !new_state.is_complete() {
                self.qos2_inbound.insert(packet_id, new_state);
                Ok(true)
            } else {
                // Flow complete, ID can be reused
                Ok(true)
            }
        } else {
            // Unknown - might be duplicate, send PUBCOMP anyway
            Ok(true)
        }
    }

    // ========================================================================
    // Offline queue management
    // ========================================================================

    /// Queue a message for offline delivery
    pub fn queue_message(&mut self, msg: QueuedMessage) -> bool {
        if self.offline_queue.len() >= self.config.max_offline_messages {
            // Queue full - drop oldest message (or drop new message based on policy)
            self.offline_queue.pop_front();
        }
        self.offline_queue.push_back(msg);
        true
    }

    /// Get next queued message
    pub fn dequeue_message(&mut self) -> Option<QueuedMessage> {
        self.offline_queue.pop_front()
    }

    /// Get number of queued messages
    pub fn queued_message_count(&self) -> usize {
        self.offline_queue.len()
    }

    /// Check if there are queued messages
    pub fn has_queued_messages(&self) -> bool {
        !self.offline_queue.is_empty()
    }

    /// Drain all queued messages
    pub fn drain_queued_messages(&mut self) -> impl Iterator<Item = QueuedMessage> + '_ {
        self.offline_queue.drain(..)
    }

    // ========================================================================
    // Session cleanup
    // ========================================================================

    /// Clear all state (for clean session or session expiry)
    pub fn clear(&mut self) {
        self.subscriptions.clear();
        self.packet_id_allocator.clear();
        self.qos1_outbound.clear();
        self.qos2_outbound.clear();
        self.qos2_inbound.clear();
        self.offline_queue.clear();
    }

    /// Get count of in-flight QoS 1/2 messages
    pub fn inflight_count(&self) -> usize {
        self.qos1_outbound.len() + self.qos2_outbound.len()
    }

    /// Check if we have capacity for more in-flight messages
    pub fn has_inflight_capacity(&self) -> bool {
        self.inflight_count() < self.config.max_inflight
    }

    /// Get session statistics
    pub fn stats(&self) -> SessionStats {
        SessionStats {
            subscription_count: self.subscriptions.len(),
            qos1_inflight: self.qos1_outbound.len(),
            qos2_outbound_inflight: self.qos2_outbound.len(),
            qos2_inbound_inflight: self.qos2_inbound.len(),
            offline_queue_size: self.offline_queue.len(),
            age: self.created_at.elapsed(),
            idle_time: self.last_active.elapsed(),
        }
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub subscription_count: usize,
    pub qos1_inflight: usize,
    pub qos2_outbound_inflight: usize,
    pub qos2_inbound_inflight: usize,
    pub offline_queue_size: usize,
    pub age: Duration,
    pub idle_time: Duration,
}
