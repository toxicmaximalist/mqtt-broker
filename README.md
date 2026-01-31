# mqtt-broker

[![Crates.io](https://img.shields.io/crates/v/mqtt-broker.svg)](https://crates.io/crates/mqtt-broker)
[![Documentation](https://docs.rs/mqtt-broker/badge.svg)](https://docs.rs/mqtt-broker)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A production-grade MQTT 3.1.1 broker implementation in Rust.

## Features

- 🚀 **High Performance** - Async I/O with Tokio, handles 10,000+ concurrent connections
- 📡 **Full MQTT 3.1.1** - All 14 packet types with complete protocol compliance
- 🎯 **QoS 0/1/2** - Complete delivery guarantees with proper state machines
- 🌳 **Topic Wildcards** - O(k) trie-based matching for `+` and `#` wildcards
- 💾 **Retained Messages** - Automatic delivery to new subscribers
- 🔄 **Session Persistence** - Clean and persistent session support
- ⏱️ **Keep-Alive** - Automatic connection timeout detection
- 🛡️ **Production Ready** - Comprehensive test suite with 176+ tests

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mqtt-broker = "0.1"
```

Or install the CLI tools:

```bash
cargo install mqtt-broker
```

## Quick Start

### Running the Broker

```bash
# Start with defaults (0.0.0.0:1883)
mqtt-broker

# Custom configuration
mqtt-broker --host 127.0.0.1 --port 1883 --max-clients 5000
```

### Publishing Messages

```bash
# Simple publish (QoS 0)
mqtt-producer sensor/temperature "25.5"

# With QoS 1
mqtt-producer -q 1 sensor/humidity "60%"

# Retained message
mqtt-producer -r sensor/status "online"
```

### Subscribing to Topics

```bash
# Subscribe to specific topic
mqtt-consumer sensor/temperature

# Single-level wildcard
mqtt-consumer 'sensor/+/temperature'

# Multi-level wildcard
mqtt-consumer 'sensor/#'

# Multiple topics with message limit
mqtt-consumer 'home/#' 'office/#' -n 100
```

## Library Usage

### Basic Server

```rust
use mqtt_broker::server::{Server, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default configuration
    let server = Server::with_defaults();
    
    println!("MQTT Broker listening on {}", server.bind_addr());
    server.run().await?;
    
    Ok(())
}
```

### Custom Configuration

```rust
use mqtt_broker::server::{Server, ServerConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ServerConfig::new("0.0.0.0:1883".parse()?);
    config.max_connections = 5000;
    config.connect_timeout = Duration::from_secs(10);
    
    let server = Server::new(config);
    server.run().await?;
    
    Ok(())
}
```

### Using the Codec Directly

```rust
use mqtt_broker::codec::{Packet, Connect, Publish, QoS, encode_packet, decode_packet};
use bytes::{Bytes, BytesMut};

// Create a CONNECT packet
let connect = Connect::new("my-client-id")
    .clean_session(true)
    .keep_alive(60);

// Encode to bytes
let mut buf = BytesMut::new();
encode_packet(&Packet::Connect(connect), &mut buf)?;

// Create a PUBLISH packet
let publish = Publish::new("sensor/temp", Bytes::from("25.5"))
    .retain(true);
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Server                               │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│  │ Client  │  │ Client  │  │ Client  │  │   ...   │        │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘        │
│       │            │            │            │              │
│       └────────────┴─────┬──────┴────────────┘              │
│                          │                                   │
│                   ┌──────▼──────┐                           │
│                   │   Router    │                           │
│                   │  ┌───────┐  │                           │
│                   │  │ Trie  │  │  Topic Matching           │
│                   │  └───────┘  │                           │
│                   └──────┬──────┘                           │
│                          │                                   │
│            ┌─────────────┼─────────────┐                    │
│            │             │             │                    │
│      ┌─────▼─────┐ ┌─────▼─────┐ ┌─────▼─────┐             │
│      │  Session  │ │  Retain   │ │Persistence│             │
│      │   Store   │ │   Store   │ │   Layer   │             │
│      └───────────┘ └───────────┘ └───────────┘             │
└─────────────────────────────────────────────────────────────┘
```

## Module Overview

| Module | Description |
|--------|-------------|
| `codec` | MQTT 3.1.1 wire protocol encoding/decoding |
| `topic_matcher` | Trie-based topic matching with wildcards |
| `session` | Connection state and QoS state machines |
| `router` | Message routing and subscription management |
| `persistence` | Pluggable storage traits and implementations |
| `transport` | TCP framing and connection handling |
| `server` | Complete async server implementation |

## Protocol Support

| Feature | Status |
|---------|--------|
| CONNECT / CONNACK | ✅ Full |
| PUBLISH (QoS 0) | ✅ Full |
| PUBLISH (QoS 1) | ✅ Full |
| PUBLISH (QoS 2) | ✅ Full |
| SUBSCRIBE / SUBACK | ✅ Full |
| UNSUBSCRIBE / UNSUBACK | ✅ Full |
| PINGREQ / PINGRESP | ✅ Full |
| DISCONNECT | ✅ Full |
| Retained Messages | ✅ Full |
| Last Will & Testament | ✅ Full |
| Clean Session | ✅ Full |
| Topic Wildcards | ✅ Full |
| Authentication | 🔄 Basic |
| TLS/SSL | 📋 Planned |
| WebSocket | 📋 Planned |
| MQTT 5.0 | 📋 Planned |

## Performance

Benchmarked on Apple M1:

- **Connections**: 10,000+ concurrent clients
- **Throughput**: ~100,000 messages/second (QoS 0)
- **Latency**: < 1ms p99 (local)
- **Memory**: ~1KB per idle connection

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific module tests
cargo test codec
cargo test router
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [MQTT 3.1.1 Specification](http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html)
- [Tokio](https://tokio.rs/) for the async runtime
- The Rust community for excellent tooling