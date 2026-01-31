//! MQTT Session Management
//!
//! This module provides session state machines for MQTT connections:
//! - Connection state machine (awaiting connect → connected → disconnected)
//! - Session state (subscriptions, pending messages, survives reconnect)
//! - QoS 1/2 delivery state machines
//! - Packet identifier allocation

mod connection;
mod packet_id;
mod qos;
mod session;

pub use connection::{ConnectionState, ConnectionStateMachine};
pub use packet_id::PacketIdAllocator;
pub use qos::{QoS1State, QoS2InboundState, QoS2OutboundState, PendingPublish};
pub use session::{Session, SessionConfig};

#[cfg(test)]
mod tests;
