//! # mqtt-broker
//!
//! A production-grade MQTT 3.1.1 broker implementation in Rust.
//!
//! ## Features
//!
//! - **Full MQTT 3.1.1 Protocol Support** - All 14 packet types with proper encoding/decoding
//! - **QoS 0/1/2** - Complete delivery guarantees with state machines
//! - **Topic Wildcards** - `+` (single-level) and `#` (multi-level) matching
//! - **Retained Messages** - Automatic delivery on subscription
//! - **Session Persistence** - Clean/persistent session handling
//! - **Keep-Alive** - Automatic timeout detection
//! - **Async/Await** - Built on Tokio for high-performance async I/O
//!
//! ## Quick Start
//!
//! ### Running the Broker
//!
//! ```bash
//! # Start the broker on default port 1883
//! mqtt-broker
//!
//! # Or with custom settings
//! mqtt-broker --host 0.0.0.0 --port 1883 --max-clients 10000
//! ```
//!
//! ### Publishing Messages
//!
//! ```bash
//! mqtt-producer sensor/temperature "25.5°C"
//! mqtt-producer -q 1 sensor/humidity "60%"  # QoS 1
//! ```
//!
//! ### Subscribing to Topics
//!
//! ```bash
//! mqtt-consumer 'sensor/#'              # All sensor topics
//! mqtt-consumer 'sensor/+/temperature'  # Single-level wildcard
//! ```
//!
//! ## Library Usage
//!
//! ```rust,no_run
//! use mqtt_broker::server::{Server, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = ServerConfig::default();
//!     let server = Server::new(config);
//!     server.run().await.expect("Server failed");
//! }
//! ```
//!
//! ## Architecture
//!
//! The broker is organized into modular components:
//!
//! - [`codec`] - MQTT 3.1.1 wire protocol encoding/decoding
//! - [`topic_matcher`] - O(k) trie-based topic matching with wildcards
//! - [`session`] - Connection and QoS state machines
//! - [`router`] - Message routing and subscription registry
//! - [`persistence`] - Pluggable storage traits
//! - [`transport`] - TCP framing and connection handling
//! - [`server`] - Full async server implementation
//!
//! ## Protocol Compliance
//!
//! This implementation targets MQTT 3.1.1 (OASIS Standard) with support for:
//!
//! | Feature | Status |
//! |---------|--------|
//! | CONNECT/CONNACK | ✅ |
//! | PUBLISH (QoS 0/1/2) | ✅ |
//! | SUBSCRIBE/SUBACK | ✅ |
//! | UNSUBSCRIBE/UNSUBACK | ✅ |
//! | PINGREQ/PINGRESP | ✅ |
//! | DISCONNECT | ✅ |
//! | Retained Messages | ✅ |
//! | Last Will & Testament | ✅ |
//! | Clean Session | ✅ |
//! | Topic Wildcards (+, #) | ✅ |

#![doc(html_root_url = "https://docs.rs/mqtt-broker/0.1.0")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

// Core modules
pub mod codec;
pub mod topic_matcher;
pub mod session;
pub mod router;
pub mod persistence;
pub mod transport;
pub mod server;

// Legacy modules (deprecated, will be removed in 0.2.0)
#[doc(hidden)]
pub mod topic;
#[doc(hidden)]
pub mod traits;
#[doc(hidden)]
pub mod broker;
#[doc(hidden)]
pub mod consumer;
#[doc(hidden)]
pub mod message;
#[doc(hidden)]
pub mod action;
#[doc(hidden)]
pub mod utils;
#[doc(hidden)]
pub mod producer;