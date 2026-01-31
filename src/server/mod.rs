//! Server module for MQTT broker orchestration.
//!
//! This module ties all components together:
//! - Client connection management
//! - Packet handling state machine
//! - Graceful shutdown

mod client;
mod handler;
mod server;

#[cfg(test)]
mod tests;

pub use client::{Client, ClientId, ClientState};
pub use handler::PacketHandler;
pub use server::{Server, ServerConfig, ServerError};
