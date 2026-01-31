//! Message router module.
//!
//! Handles subscription management and message routing using a topic trie.

mod registry;
mod retain;
mod router;

#[cfg(test)]
mod tests;

pub use registry::{Subscription, SubscriptionRegistry};
pub use retain::RetainStore;
pub use router::{MessageRouter, RouteResult, RoutedMessage};
