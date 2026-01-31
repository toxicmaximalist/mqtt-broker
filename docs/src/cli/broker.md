# mqtt-broker

The MQTT broker server.

## Usage

```bash
mqtt-broker [OPTIONS]
```

## Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--host <HOST>` | `-h` | `0.0.0.0` | Host address to bind to |
| `--port <PORT>` | `-p` | `1883` | Port to listen on |
| `--max-clients <N>` | — | `10000` | Maximum concurrent connections |
| `--help` | — | — | Print help information |

## Examples

### Start with defaults

```bash
mqtt-broker
```

Listens on `0.0.0.0:1883` (all interfaces, standard MQTT port).

### Listen on localhost only

```bash
mqtt-broker --host 127.0.0.1
```

Only accepts connections from the local machine.

### Custom port

```bash
mqtt-broker --port 8883
```

Useful when running multiple brokers or avoiding privileged ports.

### Production settings

```bash
mqtt-broker --host 0.0.0.0 --port 1883 --max-clients 50000
```

### With logging

```bash
RUST_LOG=info mqtt-broker
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

## Signals

| Signal | Action |
|--------|--------|
| `SIGINT` (Ctrl+C) | Graceful shutdown |
| `SIGTERM` | Graceful shutdown |

During graceful shutdown, the broker:
1. Stops accepting new connections
2. Sends DISCONNECT to all clients
3. Waits for clients to disconnect (up to 30 seconds)
4. Exits cleanly

## Output

```
╔══════════════════════════════════════════════════════════════╗
║              MQTT Broker v0.1.0 Starting                     ║
╠══════════════════════════════════════════════════════════════╣
║  Protocol: MQTT 3.1.1                                        ║
║  Address:  0.0.0.0:1883                                      ║
╚══════════════════════════════════════════════════════════════╝

2026-01-31T12:00:00Z  INFO mqtt_broker::server: Client connected: sensor-01 (192.168.1.100:54321)
2026-01-31T12:00:05Z  INFO mqtt_broker::server: Client connected: dashboard (192.168.1.101:54322)
2026-01-31T12:00:10Z  INFO mqtt_broker::server: Client disconnected: sensor-01
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Clean shutdown |
| `1` | Error (binding failed, invalid arguments, etc.) |
