# mqtt-consumer

Command-line tool for subscribing to MQTT topics.

## Usage

```bash
mqtt-consumer [OPTIONS] <TOPIC>...
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<TOPIC>...` | One or more topic filters to subscribe to |

Topic filters support wildcards:
- `+` — Single-level wildcard
- `#` — Multi-level wildcard

## Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--host <HOST>` | `-h` | `127.0.0.1` | Broker host address |
| `--port <PORT>` | `-p` | `1883` | Broker port |
| `--qos <QOS>` | `-q` | `0` | Maximum QoS level |
| `--client <ID>` | `-c` | auto | Client ID |
| `--count <N>` | `-n` | unlimited | Exit after N messages |
| `--help` | — | — | Print help |

## Examples

### Subscribe to a specific topic

```bash
mqtt-consumer "sensors/temperature"
```

### Subscribe with wildcards

```bash
# All topics under sensors/
mqtt-consumer "sensors/#"

# All temperature readings
mqtt-consumer "sensors/+/temperature"

# Multiple patterns
mqtt-consumer "sensors/#" "alerts/#"
```

### Limit message count

```bash
# Exit after 10 messages
mqtt-consumer -n 10 "sensors/#"
```

### With QoS 1

```bash
mqtt-consumer --qos 1 "critical/#"
```

### Custom broker

```bash
mqtt-consumer -h 192.168.1.100 -p 1883 "home/#"
```

### With specific client ID

```bash
mqtt-consumer --client "dashboard-main" "telemetry/#"
```

## Output

### Subscription confirmation

```
Connecting to 127.0.0.1:1883...
Connected as 'mqtt-consumer-12345'
Subscribed to 2 topic(s):
  sensors/# -> SuccessQoS0
  alerts/# -> SuccessQoS0

Waiting for messages... (Press Ctrl+C to exit)
```

### Receiving messages

```
[1] Topic: sensors/living-room/temperature | QoS: AtMostOnce | Retain: false
    Payload: 23.5

[2] Topic: sensors/kitchen/humidity | QoS: AtMostOnce | Retain: false
    Payload: 65%

[3] Topic: alerts/fire | QoS: AtLeastOnce | Retain: false
    Payload: Smoke detected in garage
```

### Retained message

```
[1] Topic: device/status | QoS: AtMostOnce | Retain: true
    Payload: online
```

The `Retain: true` indicates this message was stored on the broker.

### With message limit

```
[1] Topic: sensors/temp | QoS: AtMostOnce | Retain: false
    Payload: 23.5

...

[10] Topic: sensors/temp | QoS: AtMostOnce | Retain: false
    Payload: 24.1

Received 10 message(s), exiting.
Total messages received: 10
```

### Disconnection

```
^C
Received Ctrl+C, disconnecting...
Total messages received: 42
```

## Wildcard Patterns

### Single-level wildcard (`+`)

Matches exactly one topic level.

| Pattern | Matches | Doesn't Match |
|---------|---------|---------------|
| `home/+/temperature` | `home/kitchen/temperature` | `home/temperature` |
| | `home/bedroom/temperature` | `home/floor1/kitchen/temperature` |
| `+/+/status` | `device/sensor/status` | `device/status` |

### Multi-level wildcard (`#`)

Matches zero or more topic levels. Must be last in the filter.

| Pattern | Matches |
|---------|---------|
| `home/#` | `home`, `home/kitchen`, `home/kitchen/temperature` |
| `sensors/temperature/#` | `sensors/temperature`, `sensors/temperature/celsius` |
| `#` | Everything (all topics) |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Clean exit (Ctrl+C or message limit reached) |
| `1` | Error (connection failed, invalid arguments) |

## Tips

- Use quotes around wildcards to prevent shell expansion: `"sensors/#"`
- The `#` wildcard can only appear at the end of a filter
- Subscribe to `#` to see all broker traffic (debugging)
- Use `-n 1` to receive just one message and exit
