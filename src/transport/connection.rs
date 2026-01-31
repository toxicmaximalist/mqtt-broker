//! MQTT connection handling.
//!
//! Provides a high-level abstraction over a TCP connection for sending
//! and receiving MQTT packets.

use std::net::SocketAddr;
use std::time::Duration;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::codec::Packet;
use super::codec::{MqttCodec, CodecError};

/// Configuration for MQTT connections.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Maximum packet size in bytes.
    pub max_packet_size: usize,
    /// Read timeout.
    pub read_timeout: Duration,
    /// Write timeout.
    pub write_timeout: Duration,
    /// Read buffer size.
    pub read_buffer_size: usize,
    /// Write buffer size.
    pub write_buffer_size: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_packet_size: 1024 * 1024, // 1 MB
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            read_buffer_size: 8192,
            write_buffer_size: 8192,
        }
    }
}

/// An MQTT connection over TCP.
///
/// Handles reading and writing MQTT packets with buffering and timeouts.
pub struct Connection {
    /// The underlying TCP stream reader.
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    /// The underlying TCP stream writer.
    writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    /// The MQTT codec.
    codec: MqttCodec,
    /// Read buffer.
    read_buf: BytesMut,
    /// Connection configuration.
    config: ConnectionConfig,
    /// Remote peer address.
    peer_addr: SocketAddr,
    /// Whether the connection is closed.
    closed: bool,
}

impl Connection {
    /// Create a new connection from a TCP stream.
    pub fn new(stream: TcpStream, config: ConnectionConfig) -> Result<Self, ConnectionError> {
        let peer_addr = stream.peer_addr()?;
        
        // Split the stream for independent read/write
        let (read_half, write_half) = stream.into_split();
        
        let reader = BufReader::with_capacity(config.read_buffer_size, read_half);
        let writer = BufWriter::with_capacity(config.write_buffer_size, write_half);
        
        Ok(Self {
            reader,
            writer,
            codec: MqttCodec::with_max_packet_size(config.max_packet_size),
            read_buf: BytesMut::with_capacity(config.read_buffer_size),
            config,
            peer_addr,
            closed: false,
        })
    }

    /// Get the peer address.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Check if the connection is closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Read the next MQTT packet from the connection.
    ///
    /// Returns `None` if the connection is closed.
    pub async fn read_packet(&mut self) -> Result<Option<Packet>, ConnectionError> {
        if self.closed {
            return Ok(None);
        }

        loop {
            // Try to decode a packet from the buffer
            match self.codec.decode(&mut self.read_buf) {
                Ok(Some(packet)) => return Ok(Some(packet)),
                Ok(None) => {
                    // Need more data
                }
                Err(e) => {
                    self.closed = true;
                    return Err(ConnectionError::Codec(e));
                }
            }

            // Read more data with timeout
            let read_result = timeout(
                self.config.read_timeout,
                self.reader.read_buf(&mut self.read_buf),
            )
            .await;

            match read_result {
                Ok(Ok(0)) => {
                    // Connection closed
                    self.closed = true;
                    return Ok(None);
                }
                Ok(Ok(_n)) => {
                    // Data received, loop to try decoding
                }
                Ok(Err(e)) => {
                    self.closed = true;
                    return Err(ConnectionError::Io(e));
                }
                Err(_) => {
                    self.closed = true;
                    return Err(ConnectionError::ReadTimeout);
                }
            }
        }
    }

    /// Write an MQTT packet to the connection.
    pub async fn write_packet(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        if self.closed {
            return Err(ConnectionError::ConnectionClosed);
        }

        // Encode the packet
        let mut buf = BytesMut::new();
        self.codec.encode(packet, &mut buf)?;

        // Write with timeout
        let write_result = timeout(
            self.config.write_timeout,
            self.writer.write_all(&buf),
        )
        .await;

        match write_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                self.closed = true;
                Err(ConnectionError::Io(e))
            }
            Err(_) => {
                self.closed = true;
                Err(ConnectionError::WriteTimeout)
            }
        }
    }

    /// Flush the write buffer.
    pub async fn flush(&mut self) -> Result<(), ConnectionError> {
        if self.closed {
            return Err(ConnectionError::ConnectionClosed);
        }

        let flush_result = timeout(
            self.config.write_timeout,
            self.writer.flush(),
        )
        .await;

        match flush_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                self.closed = true;
                Err(ConnectionError::Io(e))
            }
            Err(_) => {
                self.closed = true;
                Err(ConnectionError::WriteTimeout)
            }
        }
    }

    /// Write a packet and flush.
    pub async fn send_packet(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        self.write_packet(packet).await?;
        self.flush().await
    }

    /// Close the connection gracefully.
    pub async fn close(&mut self) -> Result<(), ConnectionError> {
        if self.closed {
            return Ok(());
        }

        self.closed = true;
        
        // Flush any pending writes
        let _ = self.writer.flush().await;
        
        Ok(())
    }

    /// Get the maximum packet size for this connection.
    pub fn max_packet_size(&self) -> usize {
        self.config.max_packet_size
    }
}

/// Errors that can occur during connection operations.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Codec error: {0}")]
    Codec(#[from] CodecError),

    #[error("Read timeout")]
    ReadTimeout,

    #[error("Write timeout")]
    WriteTimeout,

    #[error("Connection closed")]
    ConnectionClosed,
}
