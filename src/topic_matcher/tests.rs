//! Tests for topic matching module

use super::*;

// ============================================================================
// Topic name validation tests
// ============================================================================

#[test]
fn test_valid_topic_names() {
    assert!(validate_topic_name("test").is_ok());
    assert!(validate_topic_name("test/topic").is_ok());
    assert!(validate_topic_name("a/b/c/d/e").is_ok());
    assert!(validate_topic_name("/leading/slash").is_ok());
    assert!(validate_topic_name("trailing/slash/").is_ok());
    assert!(validate_topic_name("sensors/temperature/living-room").is_ok());
    assert!(validate_topic_name("$SYS/broker/uptime").is_ok());
    assert!(validate_topic_name("unicode/日本語/topic").is_ok());
}

#[test]
fn test_empty_topic_name() {
    assert!(matches!(validate_topic_name(""), Err(TopicError::Empty)));
}

#[test]
fn test_topic_name_with_wildcards() {
    assert!(matches!(
        validate_topic_name("test/+/topic"),
        Err(TopicError::WildcardInTopicName)
    ));
    assert!(matches!(
        validate_topic_name("test/#"),
        Err(TopicError::WildcardInTopicName)
    ));
    assert!(matches!(
        validate_topic_name("+"),
        Err(TopicError::WildcardInTopicName)
    ));
    assert!(matches!(
        validate_topic_name("#"),
        Err(TopicError::WildcardInTopicName)
    ));
}

#[test]
fn test_topic_name_with_null() {
    assert!(matches!(
        validate_topic_name("test\0topic"),
        Err(TopicError::ContainsNull)
    ));
}

// ============================================================================
// Topic filter validation tests
// ============================================================================

#[test]
fn test_valid_topic_filters() {
    // No wildcards
    assert!(validate_topic_filter("test/topic").is_ok());

    // Single-level wildcard
    assert!(validate_topic_filter("+").is_ok());
    assert!(validate_topic_filter("test/+/topic").is_ok());
    assert!(validate_topic_filter("+/+/+").is_ok());
    assert!(validate_topic_filter("sensors/+/temperature").is_ok());

    // Multi-level wildcard
    assert!(validate_topic_filter("#").is_ok());
    assert!(validate_topic_filter("test/#").is_ok());
    assert!(validate_topic_filter("+/#").is_ok());
    assert!(validate_topic_filter("test/+/#").is_ok());
}

#[test]
fn test_invalid_topic_filters() {
    // # not at end
    assert!(matches!(
        validate_topic_filter("test/#/more"),
        Err(TopicError::MultiLevelNotAtEnd)
    ));

    // + not occupying entire level
    assert!(matches!(
        validate_topic_filter("test+/topic"),
        Err(TopicError::InvalidWildcard { wildcard: '+', .. })
    ));
    assert!(matches!(
        validate_topic_filter("test/+topic"),
        Err(TopicError::InvalidWildcard { wildcard: '+', .. })
    ));

    // # not occupying entire level
    assert!(matches!(
        validate_topic_filter("test#"),
        Err(TopicError::InvalidWildcard { wildcard: '#', .. })
    ));
}

// ============================================================================
// Topic matching tests
// ============================================================================

#[test]
fn test_exact_match() {
    assert!(validation::topic_matches("test/topic", "test/topic"));
    assert!(!validation::topic_matches("test/topic", "test/other"));
    assert!(!validation::topic_matches("test/topic", "test/topic/more"));
    assert!(!validation::topic_matches("test/topic/more", "test/topic"));
}

#[test]
fn test_single_level_wildcard() {
    assert!(validation::topic_matches("test/+/topic", "test/a/topic"));
    assert!(validation::topic_matches("test/+/topic", "test/xyz/topic"));
    assert!(!validation::topic_matches("test/+/topic", "test/a/b/topic"));
    assert!(!validation::topic_matches("test/+/topic", "test/topic"));

    assert!(validation::topic_matches("+/+/+", "a/b/c"));
    assert!(!validation::topic_matches("+/+/+", "a/b"));
    assert!(!validation::topic_matches("+/+/+", "a/b/c/d"));

    assert!(validation::topic_matches("+", "anything"));
    assert!(validation::topic_matches("+", ""));
}

#[test]
fn test_multi_level_wildcard() {
    assert!(validation::topic_matches("#", "anything"));
    assert!(validation::topic_matches("#", "a/b/c/d/e"));
    assert!(validation::topic_matches("#", ""));

    assert!(validation::topic_matches("test/#", "test"));
    assert!(validation::topic_matches("test/#", "test/a"));
    assert!(validation::topic_matches("test/#", "test/a/b/c"));
    assert!(!validation::topic_matches("test/#", "other/topic"));

    assert!(validation::topic_matches("+/#", "a"));
    assert!(validation::topic_matches("+/#", "a/b/c"));
}

#[test]
fn test_complex_wildcards() {
    assert!(validation::topic_matches(
        "sensors/+/+/temperature",
        "sensors/building1/floor2/temperature"
    ));
    assert!(!validation::topic_matches(
        "sensors/+/+/temperature",
        "sensors/building1/temperature"
    ));

    assert!(validation::topic_matches(
        "+/+/#",
        "a/b/c/d/e/f"
    ));
}

#[test]
fn test_system_topics() {
    // Per MQTT spec §4.7.2, wildcards match system topics at the string level.
    // The broker is responsible for NOT delivering $SYS messages to clients
    // subscribed with + or # at the root level.
    // Our basic topic_matches function does pure string matching.
    assert!(validation::topic_matches("#", "$SYS/broker/uptime"));
    assert!(validation::topic_matches("$SYS/#", "$SYS/broker/uptime"));
    // Note: + DOES match $SYS at string level - broker must filter this
    assert!(validation::topic_matches("+/broker/uptime", "$SYS/broker/uptime"));
}

// ============================================================================
// TopicFilter type tests
// ============================================================================

#[test]
fn test_topic_filter_type() {
    let filter = TopicFilter::new("test/+/topic").unwrap();
    assert!(filter.has_wildcards());
    assert_eq!(filter.as_str(), "test/+/topic");
    assert!(filter.matches("test/a/topic"));
    assert!(!filter.matches("test/a/b/topic"));

    let filter = TopicFilter::new("test/exact").unwrap();
    assert!(!filter.has_wildcards());
}

#[test]
fn test_topic_filter_invalid() {
    assert!(TopicFilter::new("").is_err());
    assert!(TopicFilter::new("test/#/invalid").is_err());
}

// ============================================================================
// TopicTrie tests
// ============================================================================

#[test]
fn test_trie_insert_and_match() {
    let mut trie: TopicTrie<&str> = TopicTrie::new();

    trie.insert("sensors/temperature", "client1");
    trie.insert("sensors/+/temperature", "client2");
    trie.insert("sensors/#", "client3");
    trie.insert("#", "client4");

    // Exact match
    let matches = trie.matches("sensors/temperature");
    assert!(matches.contains(&&"client1"));
    assert!(matches.contains(&&"client3"));
    assert!(matches.contains(&&"client4"));
    assert!(!matches.contains(&&"client2")); // + requires something between

    // Single-level wildcard match
    let matches = trie.matches("sensors/living-room/temperature");
    assert!(matches.contains(&&"client2"));
    assert!(matches.contains(&&"client3"));
    assert!(matches.contains(&&"client4"));
    assert!(!matches.contains(&&"client1"));

    // Multi-level wildcard match
    let matches = trie.matches("sensors/a/b/c/d");
    assert!(matches.contains(&&"client3"));
    assert!(matches.contains(&&"client4"));
    assert!(!matches.contains(&&"client1"));
    assert!(!matches.contains(&&"client2"));
}

#[test]
fn test_trie_remove() {
    let mut trie: TopicTrie<&str> = TopicTrie::new();

    trie.insert("test/topic", "client1");
    trie.insert("test/topic", "client2");

    assert_eq!(trie.matches("test/topic").len(), 2);

    trie.remove("test/topic", &"client1");
    let matches = trie.matches("test/topic");
    assert_eq!(matches.len(), 1);
    assert!(matches.contains(&&"client2"));
}

#[test]
fn test_trie_remove_subscriber() {
    let mut trie: TopicTrie<&str> = TopicTrie::new();

    trie.insert("test/a", "client1");
    trie.insert("test/b", "client1");
    trie.insert("test/#", "client1");
    trie.insert("test/a", "client2");

    assert_eq!(trie.len(), 4);

    let removed = trie.remove_subscriber(&"client1");
    assert_eq!(removed, 3);
    assert_eq!(trie.len(), 1);

    let matches = trie.matches("test/a");
    assert_eq!(matches.len(), 1);
    assert!(matches.contains(&&"client2"));
}

#[test]
fn test_trie_duplicate_subscription() {
    let mut trie: TopicTrie<&str> = TopicTrie::new();

    assert!(trie.insert("test/topic", "client1"));
    assert!(!trie.insert("test/topic", "client1")); // Duplicate

    assert_eq!(trie.len(), 1);
}

#[test]
fn test_trie_empty_match() {
    let trie: TopicTrie<&str> = TopicTrie::new();
    let matches = trie.matches("test/topic");
    assert!(matches.is_empty());
}

#[test]
fn test_trie_wildcard_combinations() {
    let mut trie: TopicTrie<u32> = TopicTrie::new();

    trie.insert("+", 1);
    trie.insert("+/+", 2);
    trie.insert("+/+/+", 3);
    trie.insert("#", 4);
    trie.insert("+/#", 5);

    // Single level
    let matches = trie.matches("a");
    assert!(matches.contains(&&1));
    assert!(matches.contains(&&4));
    assert!(matches.contains(&&5));
    assert!(!matches.contains(&&2));
    assert!(!matches.contains(&&3));

    // Two levels
    let matches = trie.matches("a/b");
    assert!(matches.contains(&&2));
    assert!(matches.contains(&&4));
    assert!(matches.contains(&&5));
    assert!(!matches.contains(&&1));
    assert!(!matches.contains(&&3));

    // Three levels
    let matches = trie.matches("a/b/c");
    assert!(matches.contains(&&3));
    assert!(matches.contains(&&4));
    assert!(matches.contains(&&5));
}

#[test]
fn test_trie_get_subscriptions() {
    let mut trie: TopicTrie<&str> = TopicTrie::new();

    trie.insert("test/topic", "client1");
    trie.insert("test/+/data", "client1");
    trie.insert("sensors/#", "client1");
    trie.insert("other/topic", "client2");

    let subs = trie.get_subscriptions(&"client1");
    assert_eq!(subs.len(), 3);
    assert!(subs.contains(&"test/topic".to_string()));
    assert!(subs.contains(&"test/+/data".to_string()));
    assert!(subs.contains(&"sensors/#".to_string()));
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_empty_levels() {
    // Topics with empty levels (consecutive slashes)
    assert!(validation::topic_matches("a//b", "a//b"));
    assert!(validation::topic_matches("+//+", "a//b"));
    assert!(validation::topic_matches("a/+/b", "a//b")); // Empty string matches +
}

#[test]
fn test_leading_trailing_slashes() {
    assert!(validation::topic_matches("/test", "/test"));
    assert!(validation::topic_matches("test/", "test/"));
    assert!(validation::topic_matches("/test/", "/test/"));
    assert!(validation::topic_matches("+/test", "/test")); // Leading / means empty first level
    assert!(validation::topic_matches("test/+", "test/")); // Trailing / means empty last level
}

#[test]
fn test_trie_stress() {
    let mut trie: TopicTrie<u32> = TopicTrie::new();

    // Insert many subscriptions
    for i in 0..1000 {
        trie.insert(&format!("topic/{}/subtopic", i), i);
        trie.insert(&format!("topic/{}/+", i), i + 1000);
        trie.insert(&format!("topic/{}/#", i), i + 2000);
    }

    assert_eq!(trie.len(), 3000);

    // Verify matches work
    let matches = trie.matches("topic/500/subtopic");
    assert!(matches.contains(&&500));
    assert!(matches.contains(&&1500));
    assert!(matches.contains(&&2500));

    // Remove all for one subscriber
    trie.remove_subscriber(&500);
    assert_eq!(trie.len(), 2999);
}
