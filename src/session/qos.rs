//! QoS delivery state machines

use std::time::{Duration, Instant};

use bytes::Bytes;

use crate::codec::QoS;

/// Configuration for QoS retransmission
#[derive(Debug, Clone)]
pub struct QoSConfig {
    /// Initial retry timeout
    pub retry_interval: Duration,
    /// Maximum number of retries before giving up
    pub max_retries: u32,
    /// Whether to use exponential backoff
    pub exponential_backoff: bool,
}

impl Default for QoSConfig {
    fn default() -> Self {
        QoSConfig {
            retry_interval: Duration::from_secs(5),
            max_retries: 5,
            exponential_backoff: true,
        }
    }
}

/// A pending PUBLISH message awaiting acknowledgment
#[derive(Debug, Clone)]
pub struct PendingPublish {
    /// Packet identifier
    pub packet_id: u16,
    /// Topic name
    pub topic: String,
    /// Message payload
    pub payload: Bytes,
    /// QoS level
    pub qos: QoS,
    /// Retain flag
    pub retain: bool,
    /// When the message was first sent
    pub first_sent: Instant,
    /// When the message was last sent/retried
    pub last_sent: Instant,
    /// Number of transmission attempts
    pub attempts: u32,
}

impl PendingPublish {
    /// Create a new pending publish
    pub fn new(
        packet_id: u16,
        topic: String,
        payload: Bytes,
        qos: QoS,
        retain: bool,
    ) -> Self {
        let now = Instant::now();
        PendingPublish {
            packet_id,
            topic,
            payload,
            qos,
            retain,
            first_sent: now,
            last_sent: now,
            attempts: 1,
        }
    }

    /// Check if this message should be retried
    pub fn should_retry(&self, config: &QoSConfig) -> bool {
        if self.attempts >= config.max_retries {
            return false;
        }

        let timeout = if config.exponential_backoff {
            config.retry_interval * 2u32.pow(self.attempts.saturating_sub(1).min(4))
        } else {
            config.retry_interval
        };

        self.last_sent.elapsed() >= timeout
    }

    /// Mark as retried
    pub fn mark_retried(&mut self) {
        self.last_sent = Instant::now();
        self.attempts += 1;
    }

    /// Check if max retries exceeded
    pub fn is_expired(&self, config: &QoSConfig) -> bool {
        self.attempts >= config.max_retries
    }
}

// ============================================================================
// QoS 1 State Machine (PUBLISH → PUBACK)
// ============================================================================

/// QoS 1 outbound delivery state (sender waiting for PUBACK)
#[derive(Debug, Clone)]
pub enum QoS1State {
    /// PUBLISH sent, waiting for PUBACK
    AwaitingPubAck(PendingPublish),
    /// PUBACK received, delivery complete
    Complete,
}

impl QoS1State {
    /// Create new QoS 1 state for outbound message
    pub fn new(pending: PendingPublish) -> Self {
        QoS1State::AwaitingPubAck(pending)
    }

    /// Handle PUBACK received
    pub fn on_puback(self, packet_id: u16) -> Result<Self, QoSError> {
        match self {
            QoS1State::AwaitingPubAck(pending) => {
                if pending.packet_id == packet_id {
                    Ok(QoS1State::Complete)
                } else {
                    Err(QoSError::PacketIdMismatch {
                        expected: pending.packet_id,
                        received: packet_id,
                    })
                }
            }
            QoS1State::Complete => Err(QoSError::UnexpectedAck),
        }
    }

    /// Check if delivery is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, QoS1State::Complete)
    }

    /// Get pending publish if still awaiting
    pub fn pending(&self) -> Option<&PendingPublish> {
        match self {
            QoS1State::AwaitingPubAck(p) => Some(p),
            QoS1State::Complete => None,
        }
    }

    /// Get mutable pending publish for retry updates
    pub fn pending_mut(&mut self) -> Option<&mut PendingPublish> {
        match self {
            QoS1State::AwaitingPubAck(p) => Some(p),
            QoS1State::Complete => None,
        }
    }
}

// ============================================================================
// QoS 2 State Machine (4-way handshake)
// ============================================================================

/// QoS 2 outbound delivery state (sender)
/// Flow: PUBLISH → PUBREC → PUBREL → PUBCOMP
#[derive(Debug, Clone)]
pub enum QoS2OutboundState {
    /// PUBLISH sent, waiting for PUBREC
    AwaitingPubRec(PendingPublish),
    /// PUBREC received, PUBREL sent, waiting for PUBCOMP
    AwaitingPubComp {
        packet_id: u16,
        pubrel_sent: Instant,
        attempts: u32,
    },
    /// PUBCOMP received, delivery complete
    Complete,
}

impl QoS2OutboundState {
    /// Create new QoS 2 outbound state
    pub fn new(pending: PendingPublish) -> Self {
        QoS2OutboundState::AwaitingPubRec(pending)
    }

    /// Handle PUBREC received - transition to awaiting PUBCOMP
    pub fn on_pubrec(self, packet_id: u16) -> Result<Self, QoSError> {
        match self {
            QoS2OutboundState::AwaitingPubRec(pending) => {
                if pending.packet_id == packet_id {
                    Ok(QoS2OutboundState::AwaitingPubComp {
                        packet_id,
                        pubrel_sent: Instant::now(),
                        attempts: 1,
                    })
                } else {
                    Err(QoSError::PacketIdMismatch {
                        expected: pending.packet_id,
                        received: packet_id,
                    })
                }
            }
            QoS2OutboundState::AwaitingPubComp { .. } => {
                // Duplicate PUBREC - resend PUBREL
                Ok(self)
            }
            QoS2OutboundState::Complete => Err(QoSError::UnexpectedAck),
        }
    }

    /// Handle PUBCOMP received - delivery complete
    pub fn on_pubcomp(self, packet_id: u16) -> Result<Self, QoSError> {
        match self {
            QoS2OutboundState::AwaitingPubComp { packet_id: expected, .. } => {
                if expected == packet_id {
                    Ok(QoS2OutboundState::Complete)
                } else {
                    Err(QoSError::PacketIdMismatch {
                        expected,
                        received: packet_id,
                    })
                }
            }
            QoS2OutboundState::AwaitingPubRec(_) => Err(QoSError::UnexpectedAck),
            QoS2OutboundState::Complete => Err(QoSError::UnexpectedAck),
        }
    }

    /// Check if delivery is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, QoS2OutboundState::Complete)
    }

    /// Get packet ID
    pub fn packet_id(&self) -> Option<u16> {
        match self {
            QoS2OutboundState::AwaitingPubRec(p) => Some(p.packet_id),
            QoS2OutboundState::AwaitingPubComp { packet_id, .. } => Some(*packet_id),
            QoS2OutboundState::Complete => None,
        }
    }

    /// Check if PUBREL should be retried
    pub fn should_retry_pubrel(&self, config: &QoSConfig) -> bool {
        match self {
            QoS2OutboundState::AwaitingPubComp { pubrel_sent, attempts, .. } => {
                if *attempts >= config.max_retries {
                    return false;
                }
                let timeout = if config.exponential_backoff {
                    config.retry_interval * 2u32.pow(attempts.saturating_sub(1).min(4))
                } else {
                    config.retry_interval
                };
                pubrel_sent.elapsed() >= timeout
            }
            _ => false,
        }
    }

    /// Mark PUBREL as retried
    pub fn mark_pubrel_retried(&mut self) {
        if let QoS2OutboundState::AwaitingPubComp { pubrel_sent, attempts, .. } = self {
            *pubrel_sent = Instant::now();
            *attempts += 1;
        }
    }
}

/// QoS 2 inbound delivery state (receiver)
/// Flow: receive PUBLISH → send PUBREC → receive PUBREL → send PUBCOMP
#[derive(Debug, Clone)]
pub enum QoS2InboundState {
    /// PUBLISH received, PUBREC sent, waiting for PUBREL
    /// The message has been delivered to subscribers at this point
    AwaitingPubRel {
        packet_id: u16,
        pubrec_sent: Instant,
    },
    /// PUBREL received, PUBCOMP sent, flow complete
    Complete,
}

impl QoS2InboundState {
    /// Create new QoS 2 inbound state after receiving PUBLISH
    pub fn new(packet_id: u16) -> Self {
        QoS2InboundState::AwaitingPubRel {
            packet_id,
            pubrec_sent: Instant::now(),
        }
    }

    /// Handle PUBREL received - transition to complete
    pub fn on_pubrel(self, packet_id: u16) -> Result<Self, QoSError> {
        match self {
            QoS2InboundState::AwaitingPubRel { packet_id: expected, .. } => {
                if expected == packet_id {
                    Ok(QoS2InboundState::Complete)
                } else {
                    Err(QoSError::PacketIdMismatch {
                        expected,
                        received: packet_id,
                    })
                }
            }
            QoS2InboundState::Complete => {
                // Duplicate PUBREL - resend PUBCOMP
                Ok(self)
            }
        }
    }

    /// Check if flow is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, QoS2InboundState::Complete)
    }

    /// Get packet ID
    pub fn packet_id(&self) -> Option<u16> {
        match self {
            QoS2InboundState::AwaitingPubRel { packet_id, .. } => Some(*packet_id),
            QoS2InboundState::Complete => None,
        }
    }
}

/// Errors that can occur during QoS processing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QoSError {
    /// Received packet with unexpected packet ID
    PacketIdMismatch { expected: u16, received: u16 },
    /// Received acknowledgment when not expected
    UnexpectedAck,
    /// Packet ID already in use
    DuplicatePacketId(u16),
    /// Max retries exceeded
    MaxRetriesExceeded { packet_id: u16, attempts: u32 },
}

impl std::fmt::Display for QoSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QoSError::PacketIdMismatch { expected, received } => {
                write!(f, "packet ID mismatch: expected {}, received {}", expected, received)
            }
            QoSError::UnexpectedAck => write!(f, "unexpected acknowledgment"),
            QoSError::DuplicatePacketId(id) => write!(f, "duplicate packet ID: {}", id),
            QoSError::MaxRetriesExceeded { packet_id, attempts } => {
                write!(f, "max retries exceeded for packet {}: {} attempts", packet_id, attempts)
            }
        }
    }
}

impl std::error::Error for QoSError {}
