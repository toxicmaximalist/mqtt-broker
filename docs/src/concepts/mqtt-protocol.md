# MQTT Protocol

MQTT (Message Queuing Telemetry Transport) is a lightweight publish-subscribe messaging protocol designed for constrained devices and unreliable networks.

## Version

This broker implements **MQTT 3.1.1** (OASIS Standard, 2014).

## Core Concepts

### Publish-Subscribe Model

Unlike request-response protocols (HTTP), MQTT uses a **publish-subscribe** pattern:

```
┌──────────┐          ┌────────┐          ┌──────────┐
│ Publisher│ ──────── │ Broker │ ──────── │Subscriber│
└──────────┘   Pub    └────────┘    Sub   └──────────┘
      │                    │                    │
      │  PUBLISH           │                    │
      │  topic: temp       │                    │
      │  payload: 23.5     │                    │
      │ ─────────────────► │                    │
      │                    │  PUBLISH           │
      │                    │  topic: temp       │
      │                    │  payload: 23.5     │
      │                    │ ─────────────────► │
```

- **Publishers** send messages to topics
- **Subscribers** receive messages from topics they've subscribed to
- **Broker** routes messages between publishers and subscribers

### Decoupling

MQTT provides three types of decoupling:

1. **Space** — Publishers and subscribers don't need to know each other
2. **Time** — They don't need to be online simultaneously (with persistence)
3. **Synchronization** — Operations are asynchronous

## Packet Types

MQTT defines 14 packet types:

| Type | Direction | Purpose |
|------|-----------|---------|
| CONNECT | Client → Broker | Initiate connection |
| CONNACK | Broker → Client | Acknowledge connection |
| PUBLISH | Both | Publish a message |
| PUBACK | Both | QoS 1 acknowledgment |
| PUBREC | Both | QoS 2 received |
| PUBREL | Both | QoS 2 release |
| PUBCOMP | Both | QoS 2 complete |
| SUBSCRIBE | Client → Broker | Subscribe to topics |
| SUBACK | Broker → Client | Acknowledge subscription |
| UNSUBSCRIBE | Client → Broker | Unsubscribe from topics |
| UNSUBACK | Broker → Client | Acknowledge unsubscription |
| PINGREQ | Client → Broker | Keep-alive ping |
| PINGRESP | Broker → Client | Keep-alive response |
| DISCONNECT | Client → Broker | Clean disconnect |

## Connection Lifecycle

```
Client                                  Broker
   │                                       │
   │  CONNECT                              │
   │  - client_id: "sensor-01"             │
   │  - clean_session: true                │
   │  - keep_alive: 60                     │
   │ ────────────────────────────────────► │
   │                                       │
   │                           CONNACK     │
   │                           - code: 0   │
   │ ◄──────────────────────────────────── │
   │                                       │
   │        ... publish/subscribe ...      │
   │                                       │
   │  PINGREQ                              │
   │ ────────────────────────────────────► │
   │                           PINGRESP    │
   │ ◄──────────────────────────────────── │
   │                                       │
   │  DISCONNECT                           │
   │ ────────────────────────────────────► │
   │                                       │
```

## Keep-Alive

The keep-alive mechanism ensures connections stay active:

1. Client specifies keep-alive interval in CONNECT (e.g., 60 seconds)
2. Client must send a packet within 1.5× the interval
3. If no data to send, client sends PINGREQ
4. Broker responds with PINGRESP
5. If broker receives nothing within 1.5× interval, it closes connection

## Wire Format

MQTT uses a compact binary format:

```
┌─────────────────────────────────────────────────────┐
│                    Fixed Header                      │
├──────────────────┬──────────────────────────────────┤
│  Packet Type (4) │  Flags (4)                       │
├──────────────────┴──────────────────────────────────┤
│  Remaining Length (1-4 bytes, variable int)         │
├─────────────────────────────────────────────────────┤
│                 Variable Header                      │
│            (depends on packet type)                  │
├─────────────────────────────────────────────────────┤
│                     Payload                          │
│            (depends on packet type)                  │
└─────────────────────────────────────────────────────┘
```

The variable-length integer encoding allows:
- 1 byte for values 0-127
- 2 bytes for values up to 16,383
- 3 bytes for values up to 2,097,151
- 4 bytes for values up to 268,435,455 (256 MB max packet)

## References

- [MQTT 3.1.1 Specification (OASIS)](http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html)
- [MQTT.org](https://mqtt.org/)
