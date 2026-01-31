//! MQTT Broker Library
//!
//! A production-grade MQTT 3.1.1 broker implementation.

// New modular architecture
pub mod codec;
pub mod topic_matcher;
pub mod session;
pub mod router;
pub mod persistence;
pub mod transport;
pub mod server;

// Legacy modules (to be refactored)
pub mod topic;
pub mod traits;
pub mod broker;
pub mod consumer;
pub mod message;
pub mod action;
pub mod utils;
pub mod producer;