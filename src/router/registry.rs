//! Subscription registry using topic trie for efficient matching.

use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;

use crate::codec::QoS;
use crate::topic_matcher::{TopicFilter, TopicTrie, TopicError};

/// A subscription entry representing a client's subscription to a topic filter.
#[derive(Debug, Clone, PartialEq)]
pub struct Subscription {
    /// The client ID that owns this subscription.
    pub client_id: String,
    /// The subscribed topic filter.
    pub filter: String,
    /// The maximum QoS level for this subscription.
    pub qos: QoS,
}

/// Registry for managing client subscriptions with efficient topic matching.
///
/// Uses a topic trie internally for O(k) subscription matching where k is the
/// topic depth. Thread-safe via internal locking.
pub struct SubscriptionRegistry {
    /// Topic trie mapping filters to sets of client IDs with their QoS.
    trie: RwLock<TopicTrie<HashSet<(String, QoS)>>>,
    /// Client ID -> set of subscribed filters (for cleanup).
    client_subs: RwLock<HashMap<String, HashSet<String>>>,
}

impl SubscriptionRegistry {
    /// Create a new empty subscription registry.
    pub fn new() -> Self {
        Self {
            trie: RwLock::new(TopicTrie::new()),
            client_subs: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe a client to a topic filter with the specified QoS.
    ///
    /// Returns the granted QoS (same as requested for now).
    pub fn subscribe(&self, client_id: &str, filter: &str, qos: QoS) -> Result<QoS, TopicError> {
        // Validate the filter
        let topic_filter = TopicFilter::new(filter)?;
        let filter_str = topic_filter.as_str().to_string();

        // Add to trie
        {
            let mut trie = self.trie.write();
            let entry = (client_id.to_string(), qos);
            
            if let Some(subscribers) = trie.get_mut(&filter_str) {
                // Remove any existing subscription from this client (update QoS)
                subscribers.retain(|(id, _)| id != client_id);
                subscribers.insert(entry);
            } else {
                let mut set = HashSet::new();
                set.insert(entry);
                trie.insert(&filter_str, set);
            }
        }

        // Track client's subscriptions
        {
            let mut client_subs = self.client_subs.write();
            client_subs
                .entry(client_id.to_string())
                .or_default()
                .insert(filter_str);
        }

        Ok(qos)
    }

    /// Unsubscribe a client from a topic filter.
    ///
    /// Returns true if the subscription existed and was removed.
    pub fn unsubscribe(&self, client_id: &str, filter: &str) -> bool {
        let mut removed = false;

        // Remove from trie
        {
            let mut trie = self.trie.write();
            if let Some(subscribers) = trie.get_mut(filter) {
                let before = subscribers.len();
                subscribers.retain(|(id, _)| id != client_id);
                removed = subscribers.len() < before;
                
                // If no more subscribers, remove the entry
                if subscribers.is_empty() {
                    trie.remove_filter(filter);
                }
            }
        }

        // Remove from client tracking
        if removed {
            let mut client_subs = self.client_subs.write();
            if let Some(filters) = client_subs.get_mut(client_id) {
                filters.remove(filter);
                if filters.is_empty() {
                    client_subs.remove(client_id);
                }
            }
        }

        removed
    }

    /// Remove all subscriptions for a client.
    ///
    /// Returns the list of filters the client was subscribed to.
    pub fn remove_client(&self, client_id: &str) -> Vec<String> {
        let filters = {
            let mut client_subs = self.client_subs.write();
            client_subs.remove(client_id).unwrap_or_default()
        };

        // Remove from trie
        {
            let mut trie = self.trie.write();
            for filter in &filters {
                if let Some(subscribers) = trie.get_mut(filter) {
                    subscribers.retain(|(id, _)| id != client_id);
                    if subscribers.is_empty() {
                        trie.remove_filter(filter);
                    }
                }
            }
        }

        filters.into_iter().collect()
    }

    /// Find all subscribers that match a topic name.
    ///
    /// Returns a list of (client_id, qos) pairs for all matching subscriptions.
    /// If a client has multiple matching subscriptions, only the highest QoS is returned.
    pub fn get_subscribers(&self, topic: &str) -> Vec<(String, QoS)> {
        let trie = self.trie.read();
        let matches = trie.get_matching(topic);

        // Merge results: each client gets their highest QoS across all matching subs
        let mut client_qos: HashMap<String, QoS> = HashMap::new();
        
        for subscriber_set in matches {
            for (client_id, qos) in subscriber_set {
                client_qos
                    .entry(client_id.clone())
                    .and_modify(|existing| {
                        if (*qos as u8) > (*existing as u8) {
                            *existing = *qos;
                        }
                    })
                    .or_insert(*qos);
            }
        }

        client_qos.into_iter().collect()
    }

    /// Get all subscriptions for a client.
    pub fn get_client_subscriptions(&self, client_id: &str) -> Vec<Subscription> {
        let client_subs = self.client_subs.read();
        let trie = self.trie.read();

        let Some(filters) = client_subs.get(client_id) else {
            return Vec::new();
        };

        let mut subs = Vec::new();
        for filter in filters {
            if let Some(subscribers) = trie.get(filter) {
                for (id, qos) in subscribers {
                    if id == client_id {
                        subs.push(Subscription {
                            client_id: client_id.to_string(),
                            filter: filter.clone(),
                            qos: *qos,
                        });
                    }
                }
            }
        }

        subs
    }

    /// Get the total number of unique subscriptions.
    pub fn subscription_count(&self) -> usize {
        let trie = self.trie.read();
        let mut count = 0;
        // Count all entries in the trie
        for subs in trie.iter_values() {
            count += subs.len();
        }
        count
    }

    /// Get the number of subscribed clients.
    pub fn client_count(&self) -> usize {
        self.client_subs.read().len()
    }

    /// Check if a client has any subscriptions.
    pub fn has_subscriptions(&self, client_id: &str) -> bool {
        self.client_subs.read().contains_key(client_id)
    }
}

impl Default for SubscriptionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-safe marker
unsafe impl Send for SubscriptionRegistry {}
unsafe impl Sync for SubscriptionRegistry {}
