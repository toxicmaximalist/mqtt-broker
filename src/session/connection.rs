//! Connection state machine

use std::time::{Duration, Instant};

/// Connection states per MQTT 3.1.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Waiting for CONNECT packet
    AwaitingConnect,
    /// CONNECT received, processing authentication
    Authenticating,
    /// Connection established, ready for operations
    Connected,
    /// Disconnecting (graceful or error)
    Disconnecting,
    /// Connection closed
    Closed,
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::AwaitingConnect
    }
}

/// Reasons for disconnection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Client sent DISCONNECT packet
    ClientDisconnect,
    /// Keep-alive timeout
    KeepAliveTimeout,
    /// Protocol violation
    ProtocolError(String),
    /// Authentication failed
    AuthenticationFailed,
    /// Server shutting down
    ServerShutdown,
    /// Network error
    NetworkError(String),
    /// Kicked by admin
    AdminKick,
}

/// Connection state machine managing the lifecycle of a client connection
#[derive(Debug)]
pub struct ConnectionStateMachine {
    state: ConnectionState,
    /// Client ID (set after CONNECT)
    client_id: Option<String>,
    /// Keep-alive interval from CONNECT (seconds)
    keep_alive: Duration,
    /// Last packet received timestamp
    last_packet_time: Instant,
    /// Connection established timestamp
    connected_at: Option<Instant>,
    /// Clean session flag from CONNECT
    clean_session: bool,
}

impl Default for ConnectionStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionStateMachine {
    /// Create a new connection state machine
    pub fn new() -> Self {
        ConnectionStateMachine {
            state: ConnectionState::AwaitingConnect,
            client_id: None,
            keep_alive: Duration::from_secs(60),
            last_packet_time: Instant::now(),
            connected_at: None,
            clean_session: true,
        }
    }

    /// Get current state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Get client ID (if connected)
    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    /// Get keep-alive duration
    pub fn keep_alive(&self) -> Duration {
        self.keep_alive
    }

    /// Check if this is a clean session
    pub fn is_clean_session(&self) -> bool {
        self.clean_session
    }

    /// Record that a packet was received (resets keep-alive timer)
    pub fn packet_received(&mut self) {
        self.last_packet_time = Instant::now();
    }

    /// Check if keep-alive has timed out.
    /// Per MQTT 3.1.1 §3.1.2.10: disconnect if no packet in 1.5 × keep_alive
    pub fn is_keep_alive_expired(&self) -> bool {
        if self.keep_alive.is_zero() {
            return false; // Keep-alive disabled
        }

        let timeout = self.keep_alive + self.keep_alive / 2; // 1.5x
        self.last_packet_time.elapsed() > timeout
    }

    /// Get time until keep-alive expires
    pub fn time_until_timeout(&self) -> Option<Duration> {
        if self.keep_alive.is_zero() {
            return None;
        }

        let timeout = self.keep_alive + self.keep_alive / 2;
        let elapsed = self.last_packet_time.elapsed();

        if elapsed >= timeout {
            Some(Duration::ZERO)
        } else {
            Some(timeout - elapsed)
        }
    }

    /// Transition: Received CONNECT packet
    pub fn on_connect(
        &mut self,
        client_id: String,
        clean_session: bool,
        keep_alive_secs: u16,
    ) -> Result<(), ConnectionError> {
        match self.state {
            ConnectionState::AwaitingConnect => {
                self.client_id = Some(client_id);
                self.clean_session = clean_session;
                self.keep_alive = Duration::from_secs(keep_alive_secs as u64);
                self.state = ConnectionState::Authenticating;
                self.packet_received();
                Ok(())
            }
            _ => Err(ConnectionError::UnexpectedPacket {
                state: self.state,
                packet: "CONNECT",
            }),
        }
    }

    /// Transition: Authentication completed successfully
    pub fn on_authenticated(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            ConnectionState::Authenticating => {
                self.state = ConnectionState::Connected;
                self.connected_at = Some(Instant::now());
                Ok(())
            }
            _ => Err(ConnectionError::InvalidStateTransition {
                from: self.state,
                to: ConnectionState::Connected,
            }),
        }
    }

    /// Transition: Authentication failed
    pub fn on_auth_failed(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            ConnectionState::Authenticating => {
                self.state = ConnectionState::Disconnecting;
                Ok(())
            }
            _ => Err(ConnectionError::InvalidStateTransition {
                from: self.state,
                to: ConnectionState::Disconnecting,
            }),
        }
    }

    /// Transition: Client sent DISCONNECT
    pub fn on_disconnect(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            ConnectionState::Connected => {
                self.state = ConnectionState::Disconnecting;
                Ok(())
            }
            _ => {
                // Accept disconnect from any state
                self.state = ConnectionState::Disconnecting;
                Ok(())
            }
        }
    }

    /// Transition: Connection closed
    pub fn on_closed(&mut self) {
        self.state = ConnectionState::Closed;
    }

    /// Check if connected and ready for normal operations
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    /// Check if connection is closed
    pub fn is_closed(&self) -> bool {
        self.state == ConnectionState::Closed
    }

    /// Get connection uptime
    pub fn uptime(&self) -> Option<Duration> {
        self.connected_at.map(|t| t.elapsed())
    }
}

/// Connection state machine errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionError {
    /// Received unexpected packet in current state
    UnexpectedPacket {
        state: ConnectionState,
        packet: &'static str,
    },
    /// Invalid state transition attempted
    InvalidStateTransition {
        from: ConnectionState,
        to: ConnectionState,
    },
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::UnexpectedPacket { state, packet } => {
                write!(f, "unexpected {} packet in state {:?}", packet, state)
            }
            ConnectionError::InvalidStateTransition { from, to } => {
                write!(f, "invalid state transition from {:?} to {:?}", from, to)
            }
        }
    }
}

impl std::error::Error for ConnectionError {}
