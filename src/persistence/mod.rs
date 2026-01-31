//! Persistence module for session and message storage.
//!
//! Provides storage backends for:
//! - Session state (for clean_session=false)
//! - Pending messages (QoS 1/2 inflight)
//! - Retained messages

mod memory;
mod traits;

#[cfg(test)]
mod tests;

pub use memory::MemoryStore;
pub use traits::{
    SessionStore, MessageStore, PersistenceError,
    PersistedSession, PendingMessage, Qos2State,
};
