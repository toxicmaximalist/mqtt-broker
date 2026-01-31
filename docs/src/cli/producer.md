# mqtt-producer

Command-line tool for publishing MQTT messages.

## Usage

```bash
mqtt-producer [OPTIONS] <TOPIC> <MESSAGE>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<TOPIC>` | Topic to publish to |
| `<MESSAGE>` | Message payload (text) |

## Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--host <HOST>` | `-h` | `127.0.0.1` | Broker host address |
| `--port <PORT>` | `-p` | `1883` | Broker port |
| `--qos <QOS>` | `-q` | `0` | Quality of Service (0, 1, 2) |
| `--retain` | `-r` | `false` | Retain message on broker |
| `--client <ID>` | `-c` | auto | Client ID |
| `--help` | — | — | Print help |

## Examples

### Simple publish

```bash
mqtt-producer "sensors/temperature" "23.5"
```

### Publish with QoS 1

```bash
mqtt-producer --qos 1 "sensors/humidity" "65%"
```

Waits for PUBACK confirmation from broker.

### Publish with QoS 2

```bash
mqtt-producer --qos 2 "payments/transaction" '{"id": "tx-123", "amount": 99.99}'
```

Exactly-once delivery with full 4-way handshake.

### Retained message

```bash
mqtt-producer --retain "device/status" "online"
```

New subscribers immediately receive this message.

### Custom broker

```bash
mqtt-producer -h 192.168.1.100 -p 1883 "home/lights" "on"
```

### With specific client ID

```bash
mqtt-producer --client "sensor-gateway-01" "sensors/batch" '{"temp": 23, "humidity": 65}'
```

### JSON payload

```bash
mqtt-producer "events/user" '{"event": "login", "user": "alice", "timestamp": 1706745600}'
```

### Multi-word message

```bash
mqtt-producer "notifications/alert" "Temperature exceeds threshold"
```

## Output

### Successful publish (QoS 0)

```
Connecting to 127.0.0.1:1883...
Connected as 'mqtt-producer-12345'
Published to 'sensors/temperature': 23.5
Done.
```

### Successful publish (QoS 1)

```
Connecting to 127.0.0.1:1883...
Connected as 'mqtt-producer-12345'
Published to 'sensors/temperature': 23.5
Message acknowledged (QoS 1)
Done.
```

### Successful publish (QoS 2)

```
Connecting to 127.0.0.1:1883...
Connected as 'mqtt-producer-12345'
Published to 'sensors/temperature': 23.5
Message delivered exactly once (QoS 2)
Done.
```

### Connection failure

```
Connecting to 127.0.0.1:1883...
Failed to connect: Connection refused (os error 61)
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Message published successfully |
| `1` | Error (connection failed, invalid arguments, timeout) |

## Tips

- Use single quotes for JSON payloads to avoid shell escaping issues
- Retained messages persist until replaced or cleared with empty payload
- QoS 2 has higher latency but guarantees exactly-once delivery
