# Sessions

MQTT sessions allow state to persist between client connections.

## Session Types

### Clean Session (`clean_session: true`)

A fresh start every connection:

- All subscriptions are cleared on disconnect
- All queued messages are discarded
- No state persists between connections

```bash
# Default behavior — clean session
mqtt-consumer "sensors/#"
# Disconnect and reconnect → must resubscribe
```

### Persistent Session (`clean_session: false`)

State persists across connections:

- Subscriptions are maintained
- QoS 1/2 messages are queued while offline
- Queued messages delivered on reconnect

## Session State

The broker stores the following for each session:

| Component | Description |
|-----------|-------------|
| Client ID | Unique identifier |
| Subscriptions | Topic filters and QoS levels |
| QoS 1 outbound | Messages awaiting PUBACK |
| QoS 2 outbound | Messages in QoS 2 flow |
| QoS 2 inbound | Received messages awaiting PUBREL |
| Offline queue | Messages for offline client |

## Connection Flow

### Clean Session

```
Client                                    Broker
   │                                         │
   │  CONNECT                                │
   │  client_id: "sensor-01"                 │
   │  clean_session: true                    │
   │ ──────────────────────────────────────► │
   │                                         │ Creates new session
   │                             CONNACK     │
   │                     session_present: 0  │
   │ ◄────────────────────────────────────── │
```

### Persistent Session (First Connect)

```
Client                                    Broker
   │                                         │
   │  CONNECT                                │
   │  client_id: "sensor-01"                 │
   │  clean_session: false                   │
   │ ──────────────────────────────────────► │
   │                                         │ Creates new session
   │                             CONNACK     │
   │                     session_present: 0  │
   │ ◄────────────────────────────────────── │
```

### Persistent Session (Reconnect)

```
Client                                    Broker
   │                                         │
   │  CONNECT                                │
   │  client_id: "sensor-01"                 │
   │  clean_session: false                   │
   │ ──────────────────────────────────────► │
   │                                         │ Finds existing session
   │                             CONNACK     │
   │                     session_present: 1  │
   │ ◄────────────────────────────────────── │
   │                                         │
   │                                         │ Delivers queued messages
   │                             PUBLISH     │
   │                             PUBLISH     │
   │ ◄────────────────────────────────────── │
```

## Session Takeover

If a client connects with an ID that's already connected:

1. Broker disconnects the existing client
2. New client takes over the session
3. This prevents duplicate client IDs

```
Client A (sensor-01)          Broker          Client B (sensor-01)
        │                        │                     │
        │  ◄── Connected ──►     │                     │
        │                        │                     │
        │                        │  CONNECT            │
        │                        │  client_id: sensor-01
        │                        │ ◄─────────────────── │
        │                        │                     │
   [Disconnected]                │  Takes over session │
        │                        │                     │
        │                        │  CONNACK            │
        │                        │ ────────────────────►│
```

## Offline Message Queuing

When a client with a persistent session disconnects:

1. Broker keeps the session
2. Incoming messages (matching subscriptions) are queued
3. When client reconnects, queued messages are delivered

```
Subscriber                Broker                Publisher
(offline)                    │                      │
    X                        │                      │
    │                        │  PUBLISH "sensors/t" │
    │                        │ ◄──────────────────── │
    │                        │                      │
    │                   [Queues message for subscriber]
    │                        │                      │
    │  CONNECT               │                      │
    │ ─────────────────────► │                      │
    │                        │                      │
    │  CONNACK (session: 1)  │                      │
    │ ◄───────────────────── │                      │
    │                        │                      │
    │  PUBLISH "sensors/t"   │  (queued message)    │
    │ ◄───────────────────── │                      │
```

## Queue Limits

To prevent memory exhaustion, this broker limits:

| Setting | Default | Description |
|---------|---------|-------------|
| Max offline messages | 1000 | Per client |
| Max inflight | 65535 | QoS 1/2 in progress |
| Session expiry | 24 hours | After disconnect |

When the queue is full, oldest messages are dropped (or new messages, depending on policy).

## Session Expiry

Sessions don't live forever:

- After `session_expiry` time, inactive sessions are cleaned up
- This prevents unbounded memory growth
- Default: 24 hours after last disconnect

## Client ID Rules

- Must be 1-23 characters (per MQTT 3.1.1)
- Characters: `a-z`, `A-Z`, `0-9`
- If empty, broker generates one (requires `clean_session: true`)
- Must be unique across all connected clients

## Best Practices

1. **Use persistent sessions for reliability** — Critical IoT devices
2. **Use clean sessions for ephemeral clients** — Dashboards, CLI tools
3. **Set appropriate expiry** — Balance reliability vs. memory
4. **Handle session_present** — Client should check CONNACK
5. **Use consistent client IDs** — Enables session recovery
6. **Size queues appropriately** — Based on expected offline duration

## Implementation Details

This broker uses:

- In-memory session storage (default)
- LRU eviction for session expiry
- Separate queues per QoS level
- Atomic session takeover
