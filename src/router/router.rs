//! Message router for distributing published messages to subscribers.

use crate::codec::QoS;
use crate::topic_matcher::TopicError;

use super::registry::SubscriptionRegistry;
use super::retain::{RetainStore, RetainError};

/// A message routed to a specific subscriber.
#[derive(Debug, Clone)]
pub struct RoutedMessage {
    /// Target client ID.
    pub client_id: String,
    /// The topic the message was published on.
    pub topic: String,
    /// The message payload.
    pub payload: Vec<u8>,
    /// The QoS for delivery (min of publish QoS and subscription QoS).
    pub qos: QoS,
    /// Whether this is a retained message being sent on subscribe.
    pub is_retained: bool,
}

/// Result of routing a publish message.
#[derive(Debug)]
pub struct RouteResult {
    /// Messages to deliver to subscribers.
    pub messages: Vec<RoutedMessage>,
    /// Whether the message was stored as retained.
    pub retained: bool,
    /// Number of subscribers matched.
    pub subscriber_count: usize,
}

/// The main message router combining subscription registry and retain store.
pub struct MessageRouter {
    /// Subscription registry for topic matching.
    subscriptions: SubscriptionRegistry,
    /// Retained message store.
    retain_store: RetainStore,
}

impl MessageRouter {
    /// Create a new message router.
    pub fn new() -> Self {
        Self {
            subscriptions: SubscriptionRegistry::new(),
            retain_store: RetainStore::new(),
        }
    }

    /// Create a new message router with custom retain store limits.
    pub fn with_retain_limits(max_messages: usize, max_payload_size: usize) -> Self {
        Self {
            subscriptions: SubscriptionRegistry::new(),
            retain_store: RetainStore::with_limits(max_messages, max_payload_size),
        }
    }

    /// Subscribe a client to a topic filter.
    ///
    /// Returns the granted QoS and any retained messages that match the filter.
    pub fn subscribe(
        &self,
        client_id: &str,
        filter: &str,
        qos: QoS,
    ) -> Result<(QoS, Vec<RoutedMessage>), TopicError> {
        // Add subscription
        let granted_qos = self.subscriptions.subscribe(client_id, filter, qos)?;

        // Get matching retained messages
        let retained = self.retain_store.get_matching(filter);
        let messages: Vec<RoutedMessage> = retained
            .into_iter()
            .map(|msg| RoutedMessage {
                client_id: client_id.to_string(),
                topic: msg.topic,
                payload: msg.payload,
                qos: min_qos(msg.qos, granted_qos),
                is_retained: true,
            })
            .collect();

        Ok((granted_qos, messages))
    }

    /// Unsubscribe a client from a topic filter.
    pub fn unsubscribe(&self, client_id: &str, filter: &str) -> bool {
        self.subscriptions.unsubscribe(client_id, filter)
    }

    /// Remove all subscriptions for a client.
    pub fn remove_client(&self, client_id: &str) -> Vec<String> {
        self.subscriptions.remove_client(client_id)
    }

    /// Route a published message to all matching subscribers.
    ///
    /// If `retain` is true, the message is also stored in the retain store.
    pub fn route(
        &self,
        topic: &str,
        payload: Vec<u8>,
        qos: QoS,
        retain: bool,
    ) -> Result<RouteResult, RouterError> {
        // Handle retain
        let retained = if retain {
            self.retain_store.store(topic, payload.clone(), qos)?;
            true
        } else {
            false
        };

        // Find all matching subscribers
        let subscribers = self.subscriptions.get_subscribers(topic);
        let subscriber_count = subscribers.len();

        // Create routed messages
        let messages: Vec<RoutedMessage> = subscribers
            .into_iter()
            .map(|(client_id, sub_qos)| RoutedMessage {
                client_id,
                topic: topic.to_string(),
                payload: payload.clone(),
                qos: min_qos(qos, sub_qos),
                is_retained: false,
            })
            .collect();

        Ok(RouteResult {
            messages,
            retained,
            subscriber_count,
        })
    }

    /// Route a published message, excluding the sender (for QoS 0 non-retained).
    pub fn route_excluding(
        &self,
        topic: &str,
        payload: Vec<u8>,
        qos: QoS,
        retain: bool,
        exclude_client: &str,
    ) -> Result<RouteResult, RouterError> {
        let mut result = self.route(topic, payload, qos, retain)?;
        result.messages.retain(|m| m.client_id != exclude_client);
        result.subscriber_count = result.messages.len();
        Ok(result)
    }

    /// Get the subscription registry for direct access.
    pub fn subscriptions(&self) -> &SubscriptionRegistry {
        &self.subscriptions
    }

    /// Get the retain store for direct access.
    pub fn retain_store(&self) -> &RetainStore {
        &self.retain_store
    }

    /// Get router statistics.
    pub fn stats(&self) -> RouterStats {
        RouterStats {
            subscription_count: self.subscriptions.subscription_count(),
            client_count: self.subscriptions.client_count(),
            retained_message_count: self.retain_store.count(),
            retained_payload_bytes: self.retain_store.total_payload_size(),
        }
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Router statistics.
#[derive(Debug, Clone)]
pub struct RouterStats {
    /// Total number of subscriptions.
    pub subscription_count: usize,
    /// Number of clients with subscriptions.
    pub client_count: usize,
    /// Number of retained messages.
    pub retained_message_count: usize,
    /// Total bytes of retained message payloads.
    pub retained_payload_bytes: usize,
}

/// Errors that can occur during routing.
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("Topic error: {0}")]
    Topic(#[from] TopicError),

    #[error("Retain error: {0}")]
    Retain(#[from] RetainError),
}

/// Get the minimum of two QoS levels.
fn min_qos(a: QoS, b: QoS) -> QoS {
    match (a, b) {
        (QoS::AtMostOnce, _) | (_, QoS::AtMostOnce) => QoS::AtMostOnce,
        (QoS::AtLeastOnce, _) | (_, QoS::AtLeastOnce) => QoS::AtLeastOnce,
        (QoS::ExactlyOnce, QoS::ExactlyOnce) => QoS::ExactlyOnce,
    }
}

// Thread-safe markers
unsafe impl Send for MessageRouter {}
unsafe impl Sync for MessageRouter {}
