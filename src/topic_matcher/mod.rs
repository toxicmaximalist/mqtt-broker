//! MQTT Topic Handling
//!
//! This module provides:
//! - Topic name and filter validation per MQTT 3.1.1
//! - Topic trie for efficient O(k) subscription matching
//! - Wildcard support (`+` single-level, `#` multi-level)

mod filter;
mod trie;
mod validation;

pub use filter::TopicFilter;
pub use trie::TopicTrie;
pub use validation::{validate_topic_filter, validate_topic_name, matches_filter, TopicError};

#[cfg(test)]
mod tests;
