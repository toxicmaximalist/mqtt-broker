# Configuration

## Broker Configuration

The broker can be configured via command-line arguments:

```bash
mqtt-broker [OPTIONS]
```

### Available Options

| Option | Default | Description |
|--------|---------|-------------|
| `-h, --host <HOST>` | `0.0.0.0` | Host address to bind to |
| `-p, --port <PORT>` | `1883` | Port to listen on |
| `--max-clients <N>` | `10000` | Maximum concurrent connections |
| `--help` | — | Print help information |

### Examples

```bash
# Listen on localhost only
mqtt-broker --host 127.0.0.1

# Use a different port
mqtt-broker --port 8883

# Limit connections
mqtt-broker --max-clients 1000

# Combine options
mqtt-broker --host 0.0.0.0 --port 1883 --max-clients 5000
```

## Environment Variables

Enable debug logging with the `RUST_LOG` environment variable:

```bash
# Basic info logging
RUST_LOG=info mqtt-broker

# Debug logging
RUST_LOG=debug mqtt-broker

# Trace logging (very verbose)
RUST_LOG=trace mqtt-broker

# Module-specific logging
RUST_LOG=mqtt_broker::server=debug mqtt-broker
```

## Client Configuration

### Producer Options

```bash
mqtt-producer [OPTIONS] <TOPIC> <MESSAGE>
```

| Option | Default | Description |
|--------|---------|-------------|
| `-h, --host <HOST>` | `127.0.0.1` | Broker host address |
| `-p, --port <PORT>` | `1883` | Broker port |
| `-q, --qos <QOS>` | `0` | QoS level (0, 1, 2) |
| `-r, --retain` | `false` | Retain the message |
| `-c, --client <ID>` | auto | Client ID |

### Consumer Options

```bash
mqtt-consumer [OPTIONS] <TOPIC>...
```

| Option | Default | Description |
|--------|---------|-------------|
| `-h, --host <HOST>` | `127.0.0.1` | Broker host address |
| `-p, --port <PORT>` | `1883` | Broker port |
| `-q, --qos <QOS>` | `0` | Maximum QoS level |
| `-c, --client <ID>` | auto | Client ID |
| `-n, --count <N>` | unlimited | Exit after N messages |

## Default Values

The broker uses sensible defaults for production:

| Setting | Default | Notes |
|---------|---------|-------|
| Max packet size | 256 KB | Per MQTT 3.1.1 spec |
| Connect timeout | 10 seconds | Time to receive CONNECT |
| Keep-alive multiplier | 1.5x | Per MQTT spec |
| Max inflight messages | 65535 | Per QoS 1/2 flows |
| Offline queue size | 1000 | Messages per session |
| Session expiry | 24 hours | For persistent sessions |

## Planned Configuration

Future versions will support:

- Configuration file (TOML/YAML)
- TLS/SSL settings
- Authentication backends
- Access control lists (ACL)
- Clustering configuration
