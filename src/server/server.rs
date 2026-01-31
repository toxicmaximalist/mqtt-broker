//! MQTT broker server implementation.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::timeout;

use crate::router::MessageRouter;
use crate::transport::{Connection, ConnectionConfig, ConnectionError};

use super::client::{Client, ClientId};
use super::handler::{PacketHandler, HandleResult};

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to.
    pub bind_addr: SocketAddr,
    /// Connection configuration.
    pub connection: ConnectionConfig,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
    /// Maximum time to wait for CONNECT packet.
    pub connect_timeout: Duration,
    /// Shutdown grace period.
    pub shutdown_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:1883".parse().unwrap(),
            connection: ConnectionConfig::default(),
            max_connections: 10000,
            connect_timeout: Duration::from_secs(10),
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

impl ServerConfig {
    /// Create a new server config with the specified bind address.
    pub fn new(addr: impl Into<SocketAddr>) -> Self {
        Self {
            bind_addr: addr.into(),
            ..Default::default()
        }
    }
}

/// Server errors.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),

    #[error("Server is shutting down")]
    ShuttingDown,

    #[error("Maximum connections reached")]
    MaxConnectionsReached,

    #[error("Connect timeout")]
    ConnectTimeout,

    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// MQTT broker server.
pub struct Server {
    /// Server configuration.
    config: ServerConfig,
    /// Message router.
    router: Arc<RwLock<MessageRouter>>,
    /// Active clients by client ID.
    clients: Arc<RwLock<HashMap<ClientId, Arc<RwLock<Client>>>>>,
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
}

impl Server {
    /// Create a new server with the given configuration.
    pub fn new(config: ServerConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            config,
            router: Arc::new(RwLock::new(MessageRouter::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
        }
    }

    /// Create a new server with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ServerConfig::default())
    }

    /// Get the configured bind address.
    pub fn bind_addr(&self) -> SocketAddr {
        self.config.bind_addr
    }

    /// Get the number of connected clients.
    pub fn client_count(&self) -> usize {
        self.clients.read().len()
    }

    /// Get the message router.
    pub fn router(&self) -> Arc<RwLock<MessageRouter>> {
        Arc::clone(&self.router)
    }

    /// Send a shutdown signal to all connections.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Run the server (blocking).
    pub async fn run(&self) -> Result<(), ServerError> {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        tracing::info!("MQTT broker listening on {}", self.config.bind_addr);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                // Accept new connections
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            // Check connection limit
                            if self.clients.read().len() >= self.config.max_connections {
                                tracing::warn!("Connection rejected from {}: max connections reached", addr);
                                continue;
                            }

                            // Spawn connection handler
                            self.spawn_connection_handler(stream, addr);
                        }
                        Err(e) => {
                            tracing::error!("Accept error: {}", e);
                        }
                    }
                }

                // Shutdown signal
                _ = shutdown_rx.recv() => {
                    tracing::info!("Shutdown signal received");
                    break;
                }
            }
        }

        // Graceful shutdown
        self.graceful_shutdown().await;
        
        Ok(())
    }

    /// Spawn a handler for a new connection.
    fn spawn_connection_handler(&self, stream: TcpStream, addr: SocketAddr) {
        let router = Arc::clone(&self.router);
        let clients = Arc::clone(&self.clients);
        let config = self.config.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            tracing::debug!("New connection from {}", addr);

            // Create connection wrapper
            let mut conn = match Connection::new(stream, config.connection.clone()) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to create connection from {}: {}", addr, e);
                    return;
                }
            };

            // Create client and handler
            let mut client = Client::new(addr);
            let handler = PacketHandler::new(router.clone());

            // Wait for CONNECT with timeout
            let connect_result = timeout(config.connect_timeout, conn.read_packet()).await;
            
            match connect_result {
                Ok(Ok(Some(packet))) => {
                    let result = handler.handle(&mut client, packet);
                    if let Err(e) = Self::process_handle_result_simple(&mut conn, &client.client_id, result).await {
                        tracing::debug!("Error processing CONNECT from {}: {}", addr, e);
                        return;
                    }
                }
                Ok(Ok(None)) => {
                    tracing::debug!("Connection closed before CONNECT from {}", addr);
                    return;
                }
                Ok(Err(e)) => {
                    tracing::debug!("Read error waiting for CONNECT from {}: {}", addr, e);
                    return;
                }
                Err(_) => {
                    tracing::debug!("CONNECT timeout from {}", addr);
                    return;
                }
            }

            // Check if connected successfully
            if !client.is_connected() {
                tracing::debug!("Client {} failed to connect", addr);
                return;
            }

            let client_id = client.client_id.clone();
            let clean_session = client.clean_session;
            tracing::info!("Client connected: {} ({})", client_id, addr);

            // Register client
            let client_arc = Arc::new(RwLock::new(client));
            clients.write().insert(client_id.clone(), Arc::clone(&client_arc));

            // Main packet processing loop
            loop {
                tokio::select! {
                    // Read packets
                    read_result = conn.read_packet() => {
                        match read_result {
                            Ok(Some(packet)) => {
                                // Handle packet - lock is dropped before await
                                let result = {
                                    let mut client = client_arc.write();
                                    handler.handle(&mut client, packet)
                                };

                                match Self::process_handle_result_simple(&mut conn, &client_id, result).await {
                                    Ok(should_disconnect) => {
                                        if should_disconnect {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::debug!("Error processing packet from {}: {}", client_id, e);
                                        break;
                                    }
                                }
                            }
                            Ok(None) => {
                                tracing::debug!("Connection closed by {}", client_id);
                                break;
                            }
                            Err(e) => {
                                tracing::debug!("Read error from {}: {}", client_id, e);
                                break;
                            }
                        }
                    }

                    // Shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Shutdown signal for {}", client_id);
                        break;
                    }
                }

                // Check keep-alive - lock is dropped immediately
                let keep_alive_expired = client_arc.read().is_keep_alive_expired();
                if keep_alive_expired {
                    tracing::debug!("Keep-alive expired for {}", client_id);
                    break;
                }
            }

            // Cleanup
            clients.write().remove(&client_id);
            
            // Clean subscriptions if needed
            if clean_session {
                router.write().remove_client(&client_id);
            }
            
            tracing::info!("Client disconnected: {}", client_id);
        });
    }

    /// Process the result of handling a packet (simple version without client reference).
    async fn process_handle_result_simple(
        conn: &mut Connection,
        client_id: &str,
        result: HandleResult,
    ) -> Result<bool, ConnectionError> {
        match result {
            HandleResult::Response(packet) => {
                conn.send_packet(&packet).await?;
                Ok(false)
            }
            HandleResult::Responses(packets) => {
                for packet in packets {
                    conn.send_packet(&packet).await?;
                }
                Ok(false)
            }
            HandleResult::None => Ok(false),
            HandleResult::Disconnect => Ok(true),
            HandleResult::Error(msg) => {
                tracing::warn!("Protocol error from {}: {}", client_id, msg);
                Ok(true)
            }
        }
    }

    /// Perform graceful shutdown.
    async fn graceful_shutdown(&self) {
        tracing::info!("Starting graceful shutdown...");

        // Get all clients
        let client_ids: Vec<_> = self.clients.read().keys().cloned().collect();
        
        tracing::info!("Disconnecting {} clients", client_ids.len());

        // Mark all clients as disconnecting
        for client_id in &client_ids {
            if let Some(client) = self.clients.read().get(client_id) {
                client.write().disconnect();
            }
        }

        // Wait for clients to disconnect or timeout
        let start = std::time::Instant::now();
        while !self.clients.read().is_empty() {
            if start.elapsed() > self.config.shutdown_timeout {
                tracing::warn!("Shutdown timeout, forcing disconnect");
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        tracing::info!("Graceful shutdown complete");
    }

    /// Get server statistics.
    pub fn stats(&self) -> ServerStats {
        let clients = self.clients.read();
        let router = self.router.read();
        let router_stats = router.stats();

        ServerStats {
            connected_clients: clients.len(),
            total_subscriptions: router_stats.subscription_count,
            retained_messages: router_stats.retained_message_count,
        }
    }
}

/// Server statistics.
#[derive(Debug, Clone)]
pub struct ServerStats {
    /// Number of currently connected clients.
    pub connected_clients: usize,
    /// Total number of active subscriptions.
    pub total_subscriptions: usize,
    /// Number of retained messages.
    pub retained_messages: usize,
}
