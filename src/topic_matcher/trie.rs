//! Topic Trie for efficient subscription matching
//!
//! This implements a trie (prefix tree) data structure optimized for MQTT topic matching.
//! Subscription matching is O(k) where k = number of topic levels, regardless of
//! the total number of subscriptions.

use std::collections::HashMap;


/// A trie node for topic matching
#[derive(Debug)]
struct TrieNode<V> {
    /// Subscribers at this exact node (exact match or wildcard termination)
    subscribers: Vec<V>,
    /// Single-level wildcard (+) subscribers at this level
    single_wild_subscribers: Vec<V>,
    /// Multi-level wildcard (#) subscribers at this level
    multi_wild_subscribers: Vec<V>,
    /// Children indexed by topic level string
    children: HashMap<String, TrieNode<V>>,
}

impl<V> Default for TrieNode<V> {
    fn default() -> Self {
        TrieNode {
            subscribers: Vec::new(),
            single_wild_subscribers: Vec::new(),
            multi_wild_subscribers: Vec::new(),
            children: HashMap::new(),
        }
    }
}

impl<V: Clone + PartialEq> TrieNode<V> {
    fn new() -> Self {
        Self::default()
    }

    /// Check if this node and all children are empty
    fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
            && self.single_wild_subscribers.is_empty()
            && self.multi_wild_subscribers.is_empty()
            && self.children.is_empty()
    }
}

/// Topic trie for O(k) subscription matching.
///
/// # Type Parameters
/// - `V`: The subscriber value type (e.g., client ID, callback, sender)
///
/// # Example
/// ```
/// use mqtt_broker::topic_matcher::TopicTrie;
///
/// let mut trie: TopicTrie<String> = TopicTrie::new();
///
/// // Add subscriptions
/// trie.insert("sensors/+/temperature", "client1".to_string());
/// trie.insert("sensors/#", "client2".to_string());
/// trie.insert("sensors/living-room/temperature", "client3".to_string());
///
/// // Find matching subscribers
/// let matches = trie.matches("sensors/living-room/temperature");
/// assert!(matches.contains(&&"client1".to_string()));
/// assert!(matches.contains(&&"client2".to_string()));
/// assert!(matches.contains(&&"client3".to_string()));
/// ```
#[derive(Debug)]
pub struct TopicTrie<V> {
    root: TrieNode<V>,
    /// Track total subscription count
    subscription_count: usize,
}

impl<V> Default for TopicTrie<V> {
    fn default() -> Self {
        TopicTrie {
            root: TrieNode::default(),
            subscription_count: 0,
        }
    }
}

impl<V: Clone + PartialEq + Eq> TopicTrie<V> {
    /// Create a new empty topic trie
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a subscription into the trie.
    ///
    /// # Arguments
    /// - `filter`: The topic filter (may contain + and # wildcards)
    /// - `subscriber`: The subscriber value to associate
    ///
    /// # Returns
    /// `true` if this was a new subscription, `false` if subscriber already existed
    pub fn insert(&mut self, filter: &str, subscriber: V) -> bool {
        let levels: Vec<&str> = filter.split('/').collect();
        let is_new = Self::insert_recursive(&mut self.root, &levels, 0, subscriber);
        if is_new {
            self.subscription_count += 1;
        }
        is_new
    }

    fn insert_recursive(
        node: &mut TrieNode<V>,
        levels: &[&str],
        index: usize,
        subscriber: V,
    ) -> bool {
        // Reached the end of the filter
        if index >= levels.len() {
            // Check if subscriber already exists
            if node.subscribers.contains(&subscriber) {
                return false;
            }
            node.subscribers.push(subscriber);
            return true;
        }

        let level = levels[index];

        match level {
            "#" => {
                // Multi-level wildcard - store here (must be last)
                if node.multi_wild_subscribers.contains(&subscriber) {
                    return false;
                }
                node.multi_wild_subscribers.push(subscriber);
                true
            }
            "+" => {
                // Single-level wildcard at this level
                if index == levels.len() - 1 {
                    // Last level - store as single-wild subscriber
                    if node.single_wild_subscribers.contains(&subscriber) {
                        return false;
                    }
                    node.single_wild_subscribers.push(subscriber);
                    true
                } else {
                    // More levels follow - recurse into "+" child
                    let child = node.children.entry("+".to_string()).or_default();
                    Self::insert_recursive(child, levels, index + 1, subscriber)
                }
            }
            _ => {
                // Exact level match - recurse into child
                let child = node.children.entry(level.to_string()).or_default();
                Self::insert_recursive(child, levels, index + 1, subscriber)
            }
        }
    }

    /// Remove a subscription from the trie.
    ///
    /// # Returns
    /// `true` if the subscription was found and removed
    pub fn remove(&mut self, filter: &str, subscriber: &V) -> bool {
        let levels: Vec<&str> = filter.split('/').collect();
        let removed = Self::remove_recursive(&mut self.root, &levels, 0, subscriber);
        if removed {
            self.subscription_count = self.subscription_count.saturating_sub(1);
        }
        removed
    }

    fn remove_recursive(node: &mut TrieNode<V>, levels: &[&str], index: usize, subscriber: &V) -> bool {
        if index >= levels.len() {
            // At the target node - remove from subscribers
            if let Some(pos) = node.subscribers.iter().position(|s| s == subscriber) {
                node.subscribers.swap_remove(pos);
                return true;
            }
            return false;
        }

        let level = levels[index];

        match level {
            "#" => {
                // Remove from multi-level wildcard subscribers
                if let Some(pos) = node.multi_wild_subscribers.iter().position(|s| s == subscriber) {
                    node.multi_wild_subscribers.swap_remove(pos);
                    return true;
                }
                false
            }
            "+" => {
                if index == levels.len() - 1 {
                    // Last level - remove from single-wild subscribers
                    if let Some(pos) = node.single_wild_subscribers.iter().position(|s| s == subscriber) {
                        node.single_wild_subscribers.swap_remove(pos);
                        return true;
                    }
                    false
                } else {
                    // Recurse into "+" child
                    if let Some(child) = node.children.get_mut("+") {
                        let removed = Self::remove_recursive(child, levels, index + 1, subscriber);
                        // Clean up empty children
                        if child.is_empty() {
                            node.children.remove("+");
                        }
                        removed
                    } else {
                        false
                    }
                }
            }
            _ => {
                // Recurse into exact match child
                if let Some(child) = node.children.get_mut(level) {
                    let removed = Self::remove_recursive(child, levels, index + 1, subscriber);
                    // Clean up empty children
                    if child.is_empty() {
                        node.children.remove(level);
                    }
                    removed
                } else {
                    false
                }
            }
        }
    }

    /// Find all subscribers matching a topic name.
    ///
    /// This is O(k) where k = number of topic levels.
    ///
    /// # Arguments
    /// - `topic`: The topic name to match (no wildcards allowed)
    ///
    /// # Returns
    /// Vector of references to matching subscribers
    pub fn matches(&self, topic: &str) -> Vec<&V> {
        let mut results = Vec::new();
        let levels: Vec<&str> = topic.split('/').collect();
        self.matches_recursive(&self.root, &levels, 0, &mut results);
        results
    }

    fn matches_recursive<'a>(
        &'a self,
        node: &'a TrieNode<V>,
        levels: &[&str],
        index: usize,
        results: &mut Vec<&'a V>,
    ) {
        // Multi-level wildcard matches everything from here down
        results.extend(node.multi_wild_subscribers.iter());

        // If we've consumed all topic levels
        if index >= levels.len() {
            // Add exact match subscribers
            results.extend(node.subscribers.iter());
            return;
        }

        let current_level = levels[index];
        let is_last = index == levels.len() - 1;

        // Single-level wildcard at this position
        if is_last {
            // Last level - add single-wild subscribers
            results.extend(node.single_wild_subscribers.iter());
        }

        // Check single-level wildcard child (+)
        if let Some(wild_child) = node.children.get("+") {
            self.matches_recursive(wild_child, levels, index + 1, results);
        }

        // Check exact match child
        if let Some(exact_child) = node.children.get(current_level) {
            self.matches_recursive(exact_child, levels, index + 1, results);
        }
    }

    /// Remove all subscriptions for a subscriber.
    ///
    /// This is O(n) where n = total nodes in the trie.
    /// Use sparingly (e.g., on client disconnect).
    pub fn remove_subscriber(&mut self, subscriber: &V) -> usize {
        let removed = Self::remove_subscriber_recursive(&mut self.root, subscriber);
        self.subscription_count = self.subscription_count.saturating_sub(removed);
        removed
    }

    fn remove_subscriber_recursive(node: &mut TrieNode<V>, subscriber: &V) -> usize {
        let mut removed = 0;

        // Remove from this node's subscriber lists
        if let Some(pos) = node.subscribers.iter().position(|s| s == subscriber) {
            node.subscribers.swap_remove(pos);
            removed += 1;
        }
        if let Some(pos) = node.single_wild_subscribers.iter().position(|s| s == subscriber) {
            node.single_wild_subscribers.swap_remove(pos);
            removed += 1;
        }
        if let Some(pos) = node.multi_wild_subscribers.iter().position(|s| s == subscriber) {
            node.multi_wild_subscribers.swap_remove(pos);
            removed += 1;
        }

        // Recurse into children
        let mut empty_children = Vec::new();
        for (key, child) in node.children.iter_mut() {
            removed += Self::remove_subscriber_recursive(child, subscriber);
            if child.is_empty() {
                empty_children.push(key.clone());
            }
        }

        // Clean up empty children
        for key in empty_children {
            node.children.remove(&key);
        }

        removed
    }

    /// Get the total number of subscriptions
    pub fn len(&self) -> usize {
        self.subscription_count
    }

    /// Check if the trie is empty
    pub fn is_empty(&self) -> bool {
        self.subscription_count == 0
    }

    /// Get all topic filters for a subscriber.
    ///
    /// This is O(n) - use sparingly.
    pub fn get_subscriptions(&self, subscriber: &V) -> Vec<String> {
        let mut results = Vec::new();
        Self::get_subscriptions_recursive(&self.root, subscriber, String::new(), &mut results);
        results
    }

    fn get_subscriptions_recursive(
        node: &TrieNode<V>,
        subscriber: &V,
        prefix: String,
        results: &mut Vec<String>,
    ) {
        // Check this node
        if node.subscribers.contains(subscriber) {
            results.push(if prefix.is_empty() {
                prefix.clone()
            } else {
                prefix.trim_end_matches('/').to_string()
            });
        }

        if node.single_wild_subscribers.contains(subscriber) {
            let filter = if prefix.is_empty() {
                "+".to_string()
            } else {
                format!("{}+", prefix)
            };
            results.push(filter);
        }

        if node.multi_wild_subscribers.contains(subscriber) {
            let filter = if prefix.is_empty() {
                "#".to_string()
            } else {
                format!("{}#", prefix)
            };
            results.push(filter);
        }

        // Recurse into children
        for (level, child) in &node.children {
            let new_prefix = if prefix.is_empty() {
                format!("{}/", level)
            } else {
                format!("{}{}/", prefix, level)
            };
            Self::get_subscriptions_recursive(child, subscriber, new_prefix, results);
        }
    }

    /// Get a reference to the value at an exact filter path.
    ///
    /// This does NOT perform wildcard matching - it looks up the exact filter string.
    pub fn get(&self, filter: &str) -> Option<&V> {
        let levels: Vec<&str> = filter.split('/').collect();
        Self::get_recursive(&self.root, &levels, 0)
    }

    fn get_recursive<'a>(node: &'a TrieNode<V>, levels: &[&str], index: usize) -> Option<&'a V> {
        if index >= levels.len() {
            return node.subscribers.first();
        }

        let level = levels[index];

        match level {
            "#" => node.multi_wild_subscribers.first(),
            "+" if index == levels.len() - 1 => node.single_wild_subscribers.first(),
            "+" => {
                node.children.get("+").and_then(|child| {
                    Self::get_recursive(child, levels, index + 1)
                })
            }
            _ => {
                node.children.get(level).and_then(|child| {
                    Self::get_recursive(child, levels, index + 1)
                })
            }
        }
    }

    /// Get a mutable reference to the value at an exact filter path.
    pub fn get_mut(&mut self, filter: &str) -> Option<&mut V> {
        let levels: Vec<&str> = filter.split('/').collect();
        Self::get_mut_recursive(&mut self.root, &levels, 0)
    }

    fn get_mut_recursive<'a>(node: &'a mut TrieNode<V>, levels: &[&str], index: usize) -> Option<&'a mut V> {
        if index >= levels.len() {
            return node.subscribers.first_mut();
        }

        let level = levels[index];

        match level {
            "#" => node.multi_wild_subscribers.first_mut(),
            "+" if index == levels.len() - 1 => node.single_wild_subscribers.first_mut(),
            "+" => {
                node.children.get_mut("+").and_then(|child| {
                    Self::get_mut_recursive(child, levels, index + 1)
                })
            }
            _ => {
                node.children.get_mut(level).and_then(|child| {
                    Self::get_mut_recursive(child, levels, index + 1)
                })
            }
        }
    }

    /// Get all values matching a topic (including wildcards).
    ///
    /// Unlike `matches()` which returns references to individual values,
    /// this is useful when V is a collection type like `HashSet<T>`.
    pub fn get_matching(&self, topic: &str) -> Vec<&V> {
        self.matches(topic)
    }

    /// Remove the subscription at an exact filter path (regardless of subscriber).
    ///
    /// Returns true if something was removed.
    pub fn remove_filter(&mut self, filter: &str) -> bool {
        let levels: Vec<&str> = filter.split('/').collect();
        let removed = Self::remove_filter_recursive(&mut self.root, &levels, 0);
        if removed > 0 {
            self.subscription_count = self.subscription_count.saturating_sub(removed);
        }
        removed > 0
    }

    fn remove_filter_recursive(node: &mut TrieNode<V>, levels: &[&str], index: usize) -> usize {
        if index >= levels.len() {
            let count = node.subscribers.len();
            node.subscribers.clear();
            return count;
        }

        let level = levels[index];

        match level {
            "#" => {
                let count = node.multi_wild_subscribers.len();
                node.multi_wild_subscribers.clear();
                count
            }
            "+" if index == levels.len() - 1 => {
                let count = node.single_wild_subscribers.len();
                node.single_wild_subscribers.clear();
                count
            }
            "+" => {
                if let Some(child) = node.children.get_mut("+") {
                    let removed = Self::remove_filter_recursive(child, levels, index + 1);
                    if child.is_empty() {
                        node.children.remove("+");
                    }
                    removed
                } else {
                    0
                }
            }
            _ => {
                if let Some(child) = node.children.get_mut(level) {
                    let removed = Self::remove_filter_recursive(child, levels, index + 1);
                    if child.is_empty() {
                        node.children.remove(level);
                    }
                    removed
                } else {
                    0
                }
            }
        }
    }

    /// Iterate over all values in the trie.
    pub fn iter_values(&self) -> impl Iterator<Item = &V> {
        let mut values = Vec::new();
        Self::collect_values(&self.root, &mut values);
        values.into_iter()
    }

    fn collect_values<'a>(node: &'a TrieNode<V>, values: &mut Vec<&'a V>) {
        values.extend(node.subscribers.iter());
        values.extend(node.single_wild_subscribers.iter());
        values.extend(node.multi_wild_subscribers.iter());

        for child in node.children.values() {
            Self::collect_values(child, values);
        }
    }
}
