//! Topic filter type with validation

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use crate::topic_matcher::validation::{validate_topic_filter, validate_topic_name, TopicError};

/// A validated topic filter (for subscriptions).
///
/// Topic filters may contain wildcards:
/// - `+` matches a single topic level
/// - `#` matches multiple levels (must be at end)
#[derive(Debug, Clone, Eq)]
pub struct TopicFilter {
    filter: String,
    has_wildcards: bool,
}

impl TopicFilter {
    /// Create a new topic filter with validation.
    pub fn new(filter: impl Into<String>) -> Result<Self, TopicError> {
        let filter = filter.into();
        validate_topic_filter(&filter)?;

        let has_wildcards = filter.contains('+') || filter.contains('#');

        Ok(TopicFilter {
            filter,
            has_wildcards,
        })
    }

    /// Create without validation (use carefully).
    ///
    /// # Safety
    /// The caller must ensure the filter is valid.
    pub fn new_unchecked(filter: impl Into<String>) -> Self {
        let filter = filter.into();
        let has_wildcards = filter.contains('+') || filter.contains('#');
        TopicFilter {
            filter,
            has_wildcards,
        }
    }

    /// Check if this filter contains wildcards
    pub fn has_wildcards(&self) -> bool {
        self.has_wildcards
    }

    /// Get the filter as a string slice
    pub fn as_str(&self) -> &str {
        &self.filter
    }

    /// Get the topic levels
    pub fn levels(&self) -> impl Iterator<Item = &str> {
        self.filter.split('/')
    }

    /// Check if this filter matches a topic name
    pub fn matches(&self, topic: &str) -> bool {
        super::validation::topic_matches(&self.filter, topic)
    }

    /// Convert to owned String
    pub fn into_string(self) -> String {
        self.filter
    }
}

impl Deref for TopicFilter {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.filter
    }
}

impl PartialEq for TopicFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
    }
}

impl Hash for TopicFilter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.filter.hash(state);
    }
}

impl fmt::Display for TopicFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.filter)
    }
}

impl AsRef<str> for TopicFilter {
    fn as_ref(&self) -> &str {
        &self.filter
    }
}

/// A validated topic name (for publishing).
///
/// Topic names cannot contain wildcards.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicName {
    name: String,
}

impl TopicName {
    /// Create a new topic name with validation.
    pub fn new(name: impl Into<String>) -> Result<Self, TopicError> {
        let name = name.into();
        validate_topic_name(&name)?;
        Ok(TopicName { name })
    }

    /// Create without validation (use carefully).
    pub fn new_unchecked(name: impl Into<String>) -> Self {
        TopicName { name: name.into() }
    }

    /// Get the name as a string slice
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// Get the topic levels
    pub fn levels(&self) -> impl Iterator<Item = &str> {
        self.name.split('/')
    }

    /// Check if this is a system topic (starts with $)
    pub fn is_system(&self) -> bool {
        self.name.starts_with('$')
    }

    /// Convert to owned String
    pub fn into_string(self) -> String {
        self.name
    }
}

impl Deref for TopicName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl fmt::Display for TopicName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl AsRef<str> for TopicName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
