# Architecture Overview

This broker is built with modularity and testability as core principles.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           TCP Listener                              │
│                         (tokio::net::TcpListener)                   │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          Transport Layer                            │
│                                                                     │
│  ┌─────────────────────┐         ┌─────────────────────┐           │
│  │    MqttCodec        │         │    Connection       │           │
│  │ (Framed<TcpStream>) │ ───────►│   (async wrapper)   │           │
│  └─────────────────────┘         └─────────────────────┘           │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          Codec Layer                                │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────────┐  │
│  │     Decoder     │  │     Encoder     │  │    Packet Types    │  │
│  │ (MQTT 3.1.1)    │  │ (MQTT 3.1.1)    │  │ (16 packet types)  │  │
│  └─────────────────┘  └─────────────────┘  └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          Server Layer                               │
│                                                                     │
│  ┌─────────────────────┐         ┌─────────────────────────────┐   │
│  │    MqttServer       │         │     ClientHandler           │   │
│  │  (accept loop)      │────────►│  (per-connection state)     │   │
│  └─────────────────────┘         └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          Session Layer                              │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────────┐  │
│  │     Session     │  │   QoS State     │  │  Packet ID Alloc   │  │
│  │  (client state) │  │   Machines      │  │   (1-65535)        │  │
│  └─────────────────┘  └─────────────────┘  └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          Router Layer                               │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────────┐  │
│  │     Router      │  │    Registry     │  │   Retain Store     │  │
│  │(msg distribution)│  │ (subscriptions) │  │ (retained msgs)    │  │
│  └─────────────────┘  └─────────────────┘  └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Topic Matcher Layer                           │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────────┐  │
│  │   TopicTrie     │  │  TopicFilter    │  │   Validation       │  │
│  │   (O(levels))   │  │  (+, #, exact)  │  │   (UTF-8, limits)  │  │
│  └─────────────────┘  └─────────────────┘  └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Persistence Layer                            │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────────┐  │
│  │  Storage Trait  │  │  MemoryStore    │  │  (Future: Disk)    │  │
│  │   (abstract)    │  │  (default)      │  │                    │  │
│  └─────────────────┘  └─────────────────┘  └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Publish Flow (QoS 1)

```
Publisher               Broker                       Subscriber
    │                      │                             │
    │  PUBLISH (QoS 1)     │                             │
    │ ────────────────────►│                             │
    │                      │                             │
    │                      │──── Route to matching ─────►│
    │                      │     subscribers             │
    │                      │                             │
    │                      │                    PUBLISH  │
    │                      │─────────────────────────────►
    │                      │                             │
    │                      │◄──────────────────── PUBACK │
    │                      │                             │
    │               PUBACK │                             │
    │◄─────────────────────│                             │
```

### Subscribe Flow

```
Client                  Broker
   │                       │
   │  SUBSCRIBE            │
   │  topic: "sensors/#"   │
   │  qos: 1               │
   │ ─────────────────────►│
   │                       │
   │                       │  ┌──────────────────────┐
   │                       │  │ 1. Validate filter   │
   │                       │  │ 2. Add to trie       │
   │                       │  │ 3. Store in session  │
   │                       │  │ 4. Check retain      │
   │                       │  └──────────────────────┘
   │                       │
   │  SUBACK               │
   │  granted: [1]         │
   │◄──────────────────────│
   │                       │
   │  PUBLISH (retained)   │
   │◄──────────────────────│
```

## Concurrency Model

### Task Structure

```
                    ┌─────────────────────────┐
                    │      Main Task          │
                    │   (accept connections)  │
                    └───────────┬─────────────┘
                                │
            ┌───────────────────┼───────────────────┐
            ▼                   ▼                   ▼
   ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
   │  Client Task   │  │  Client Task   │  │  Client Task   │
   │  (handle one)  │  │  (handle one)  │  │  (handle one)  │
   └────────────────┘  └────────────────┘  └────────────────┘
```

Each client gets its own async task for:
- Reading packets from socket
- Processing MQTT protocol
- Writing responses

### Shared State

```
                    ┌────────────────────────┐
                    │    Shared State        │
                    │    (Arc<Router>)       │
                    │                        │
                    │  ┌──────────────────┐  │
                    │  │ Subscription Trie│  │
                    │  │   (RwLock)       │  │
                    │  └──────────────────┘  │
                    │  ┌──────────────────┐  │
                    │  │ Session Store    │  │
                    │  │   (RwLock)       │  │
                    │  └──────────────────┘  │
                    │  ┌──────────────────┐  │
                    │  │ Retain Store     │  │
                    │  │   (RwLock)       │  │
                    │  └──────────────────┘  │
                    └────────────────────────┘
                           ▲    ▲    ▲
                           │    │    │
      ┌────────────────────┘    │    └────────────────────┐
      │                         │                         │
 Client Task 1            Client Task 2             Client Task 3
```

Lock granularity:
- **Read locks** for subscription lookups (fast, parallel)
- **Write locks** for subscribe/unsubscribe (rare)
- Per-session locks for client state

## Error Handling Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                        Error Hierarchy                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  MqttError (top-level)                                          │
│    ├── CodecError (packet encoding/decoding)                    │
│    │     ├── InvalidPacketType                                  │
│    │     ├── MalformedRemainingLength                           │
│    │     ├── InvalidUtf8                                        │
│    │     └── PacketTooLarge                                     │
│    │                                                            │
│    ├── ProtocolError (MQTT violations)                          │
│    │     ├── InvalidClientId                                    │
│    │     ├── InvalidTopicFilter                                 │
│    │     └── PacketIdInUse                                      │
│    │                                                            │
│    ├── SessionError (session management)                        │
│    │     ├── SessionNotFound                                    │
│    │     └── SessionTakeover                                    │
│    │                                                            │
│    └── IoError (network)                                        │
│          └── (std::io::Error wrapped)                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Memory Management

### Per-Connection Memory

| Component | Typical Size |
|-----------|--------------|
| TCP buffer (read) | 8 KB |
| TCP buffer (write) | 8 KB |
| Session state | ~1 KB |
| Subscription entries | ~100 bytes each |
| Inflight messages | ~1 KB each |

### Global Memory

| Component | Typical Size |
|-----------|--------------|
| Subscription trie | O(unique topic levels) |
| Session store | O(connected clients) |
| Retain store | O(retained messages) |
| Offline queues | O(queued messages) |

## Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|-----------------|-------|
| Topic match | O(topic levels) | Trie traversal |
| Subscribe | O(filter levels) | Trie insertion |
| Unsubscribe | O(filter levels) | Trie removal |
| Publish routing | O(subscribers) | For each match |
| Session lookup | O(1) | HashMap |
| Packet decode | O(packet size) | Linear scan |
| Packet encode | O(packet size) | Linear write |

## Design Decisions

### Why Trie for Topics?

- Wildcards require prefix matching
- O(levels) vs O(total subscriptions)
- Memory efficient for shared prefixes
- Natural hierarchy representation

### Why In-Memory Storage?

- Simplicity for learning/prototyping
- Sufficient for most use cases
- Trait abstraction allows disk later
- Avoids I/O latency

### Why One Task Per Client?

- Simple mental model
- No complex message passing
- Backpressure handled naturally
- Easy cleanup on disconnect

### Why RwLock vs Channel?

- Subscriptions are read-heavy
- Parallel reads scale well
- Writes are rare (subscribe/unsubscribe)
- Simpler than actor model
