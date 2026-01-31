//! Topic validation per MQTT 3.1.1 specification

use thiserror::Error;

/// Errors related to topic names and filters
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TopicError {
    /// Topic is empty
    #[error("topic cannot be empty")]
    Empty,

    /// Topic exceeds maximum length (65535 bytes)
    #[error("topic exceeds maximum length of 65535 bytes")]
    TooLong,

    /// Topic contains null character (U+0000)
    #[error("topic cannot contain null character")]
    ContainsNull,

    /// Wildcard in invalid position
    #[error("wildcard '{wildcard}' in invalid position: {reason}")]
    InvalidWildcard {
        wildcard: char,
        reason: &'static str,
    },

    /// Topic name contains wildcard (not allowed in PUBLISH)
    #[error("topic name cannot contain wildcards")]
    WildcardInTopicName,

    /// Multi-level wildcard not at end
    #[error("multi-level wildcard '#' must be last character")]
    MultiLevelNotAtEnd,

    /// Invalid UTF-8
    #[error("topic must be valid UTF-8")]
    InvalidUtf8,
}

/// Validate a topic name (used in PUBLISH packets).
/// Topic names MUST NOT contain wildcards.
///
/// Per MQTT 3.1.1 §4.7:
/// - Topic names are UTF-8 encoded strings
/// - Must be at least 1 character
/// - Must not exceed 65535 bytes
/// - Must not contain null character (U+0000)
/// - Must not contain wildcards (+ or #)
pub fn validate_topic_name(topic: &str) -> Result<(), TopicError> {
    // Check empty
    if topic.is_empty() {
        return Err(TopicError::Empty);
    }

    // Check length (UTF-8 byte length)
    if topic.len() > 65535 {
        return Err(TopicError::TooLong);
    }

    // Check for null character
    if topic.contains('\0') {
        return Err(TopicError::ContainsNull);
    }

    // Check for wildcards (not allowed in topic names)
    if topic.contains('+') || topic.contains('#') {
        return Err(TopicError::WildcardInTopicName);
    }

    Ok(())
}

/// Validate a topic filter (used in SUBSCRIBE packets).
/// Topic filters MAY contain wildcards.
///
/// Per MQTT 3.1.1 §4.7:
/// - `+` matches exactly one topic level
/// - `#` matches any number of levels (must be last character)
/// - Wildcards must occupy an entire level
///
/// Valid examples:
/// - `sport/tennis/player1`
/// - `sport/+/player1`
/// - `sport/#`
/// - `+/tennis/#`
/// - `#` (matches everything)
///
/// Invalid examples:
/// - `sport+` (+ must occupy entire level)
/// - `sport/tennis#` (# must occupy entire level)
/// - `sport/#/ranking` (# must be last)
pub fn validate_topic_filter(filter: &str) -> Result<(), TopicError> {
    // Check empty
    if filter.is_empty() {
        return Err(TopicError::Empty);
    }

    // Check length
    if filter.len() > 65535 {
        return Err(TopicError::TooLong);
    }

    // Check for null character
    if filter.contains('\0') {
        return Err(TopicError::ContainsNull);
    }

    // Validate wildcard positions
    let levels: Vec<&str> = filter.split('/').collect();

    for (i, level) in levels.iter().enumerate() {
        let is_last = i == levels.len() - 1;

        // Check single-level wildcard
        if level.contains('+') {
            if *level != "+" {
                return Err(TopicError::InvalidWildcard {
                    wildcard: '+',
                    reason: "must occupy entire topic level",
                });
            }
        }

        // Check multi-level wildcard
        if level.contains('#') {
            if *level != "#" {
                return Err(TopicError::InvalidWildcard {
                    wildcard: '#',
                    reason: "must occupy entire topic level",
                });
            }
            if !is_last {
                return Err(TopicError::MultiLevelNotAtEnd);
            }
        }
    }

    Ok(())
}

/// Check if a topic filter matches a topic name.
///
/// This is the core matching algorithm per MQTT 3.1.1 §4.7.
pub fn topic_matches(filter: &str, topic: &str) -> bool {
    // Special case: filter "#" matches everything
    if filter == "#" {
        return true;
    }

    let filter_levels: Vec<&str> = filter.split('/').collect();
    let topic_levels: Vec<&str> = topic.split('/').collect();

    let mut fi = 0; // filter index
    let mut ti = 0; // topic index

    while fi < filter_levels.len() {
        let filter_level = filter_levels[fi];

        // Multi-level wildcard matches rest
        if filter_level == "#" {
            return true;
        }

        // No more topic levels to match
        if ti >= topic_levels.len() {
            return false;
        }

        let topic_level = topic_levels[ti];

        // Single-level wildcard matches exactly one level
        if filter_level == "+" {
            fi += 1;
            ti += 1;
            continue;
        }

        // Exact match required
        if filter_level != topic_level {
            return false;
        }

        fi += 1;
        ti += 1;
    }

    // Both must be exhausted for a match (unless filter ended with #)
    fi == filter_levels.len() && ti == topic_levels.len()
}

/// Split a topic into its levels
pub fn topic_levels(topic: &str) -> Vec<&str> {
    topic.split('/').collect()
}

/// Check if a topic filter contains wildcards
pub fn has_wildcards(filter: &str) -> bool {
    filter.contains('+') || filter.contains('#')
}

/// Check if a topic starts with $ (system topic)
pub fn is_system_topic(topic: &str) -> bool {
    topic.starts_with('$')
}

/// Check if a topic matches a filter (alias for topic_matches with reversed arguments).
///
/// This is a convenience function for checking if a topic matches a subscription filter.
pub fn matches_filter(topic: &str, filter: &str) -> bool {
    topic_matches(filter, topic)
}
