# mqtt-broker

A production-grade **MQTT 3.1.1** broker implementation in Rust.

## Features

- 🚀 **High Performance** — Async I/O with Tokio, handles 10,000+ concurrent connections
- 📡 **Full MQTT 3.1.1** — All 14 packet types with complete protocol compliance
- 🎯 **QoS 0/1/2** — Complete delivery guarantees with proper state machines
- 🌳 **Topic Wildcards** — O(k) trie-based matching for `+` and `#` wildcards
- 💾 **Retained Messages** — Automatic delivery to new subscribers
- 🔄 **Session Persistence** — Clean and persistent session support
- ⏱️ **Keep-Alive** — Automatic connection timeout detection
- 🛡️ **Production Ready** — Comprehensive test suite with 176+ tests

## What is MQTT?

MQTT (Message Queuing Telemetry Transport) is a lightweight publish-subscribe messaging protocol designed for constrained devices and low-bandwidth, high-latency networks. It's widely used in:

- **IoT (Internet of Things)** — Sensor data collection, device control
- **Home Automation** — Smart home devices, lighting, thermostats
- **Mobile Applications** — Push notifications, real-time messaging
- **Industrial Systems** — SCADA, telemetry, monitoring

## Quick Example

```bash
# Terminal 1: Start the broker
mqtt-broker

# Terminal 2: Subscribe to temperature readings
mqtt-consumer "sensors/+/temperature"

# Terminal 3: Publish a temperature reading
mqtt-producer "sensors/living-room/temperature" "23.5°C"
```

## Next Steps

- [Installation](./getting-started/installation.md) — Get mqtt-broker running
- [Quick Start](./getting-started/quickstart.md) — Your first pub/sub in 5 minutes
- [MQTT Concepts](./concepts/mqtt-protocol.md) — Learn the protocol basics
