# Quick Start

This guide will get you up and running with mqtt-broker in 5 minutes.

## Step 1: Start the Broker

Open a terminal and start the broker:

```bash
mqtt-broker
```

You should see:

```
╔══════════════════════════════════════════════════════════════╗
║              MQTT Broker v0.1.0 Starting                     ║
╠══════════════════════════════════════════════════════════════╣
║  Protocol: MQTT 3.1.1                                        ║
║  Address:  0.0.0.0:1883                                      ║
╚══════════════════════════════════════════════════════════════╝
```

The broker is now listening for connections on port 1883.

## Step 2: Subscribe to a Topic

Open a **second terminal** and subscribe to a topic:

```bash
mqtt-consumer "hello/world"
```

Output:

```
Connecting to 127.0.0.1:1883...
Connected as 'mqtt-consumer-12345'
Subscribed to 1 topic(s):
  hello/world -> SuccessQoS0

Waiting for messages... (Press Ctrl+C to exit)
```

The consumer is now waiting for messages on the `hello/world` topic.

## Step 3: Publish a Message

Open a **third terminal** and publish a message:

```bash
mqtt-producer "hello/world" "Hello, MQTT!"
```

Output:

```
Connecting to 127.0.0.1:1883...
Connected as 'mqtt-producer-12346'
Published to 'hello/world': Hello, MQTT!
Done.
```

## Step 4: See the Message

Back in the **consumer terminal**, you should see:

```
[1] Topic: hello/world | QoS: AtMostOnce | Retain: false
    Payload: Hello, MQTT!
```

🎉 **Congratulations!** You've just sent your first MQTT message.

## Using Wildcards

MQTT supports powerful topic wildcards:

### Single-Level Wildcard (`+`)

Subscribe to all rooms' temperature:

```bash
mqtt-consumer "home/+/temperature"
```

This matches:
- `home/kitchen/temperature` ✅
- `home/bedroom/temperature` ✅
- `home/kitchen/humidity` ❌

### Multi-Level Wildcard (`#`)

Subscribe to everything under `home/`:

```bash
mqtt-consumer "home/#"
```

This matches:
- `home/kitchen/temperature` ✅
- `home/bedroom/humidity` ✅
- `home/garage/door/status` ✅

## Using QoS Levels

Specify Quality of Service for reliable delivery:

```bash
# QoS 1 - At least once delivery
mqtt-producer "sensors/critical" "alert!" --qos 1

# QoS 2 - Exactly once delivery
mqtt-producer "transactions/payment" "confirmed" --qos 2
```

## Retained Messages

Retain the last message on a topic:

```bash
# Publish with retain flag
mqtt-producer "device/status" "online" --retain

# New subscribers will immediately receive "online"
mqtt-consumer "device/status"
```

## Next Steps

- [Configuration](./configuration.md) — Customize broker settings
- [Topics & Wildcards](../concepts/topics.md) — Deep dive into topic patterns
- [Quality of Service](../concepts/qos.md) — Understanding QoS levels
