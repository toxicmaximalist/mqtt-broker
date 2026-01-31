# Module Reference

This broker is organized into well-defined modules, each with a single responsibility.

## Module Dependency Graph

```
                    ┌─────────────────┐
                    │    bin/broker   │
                    │  bin/producer   │
                    │  bin/consumer   │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │     server      │
                    └────────┬────────┘
                             │
          ┌──────────────────┼──────────────────┐
          ▼                  ▼                  ▼
   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
   │   session   │    │   router    │    │  transport  │
   └──────┬──────┘    └──────┬──────┘    └──────┬──────┘
          │                  │                  │
          │           ┌──────┴──────┐           │
          │           ▼             ▼           │
          │    ┌───────────┐ ┌───────────┐      │
          │    │topic_match│ │persistence│      │
          │    └───────────┘ └───────────┘      │
          │                                     │
          └─────────────────┬───────────────────┘
                            ▼
                     ┌─────────────┐
                     │    codec    │
                     └─────────────┘
```

---

## `codec` — MQTT 3.1.1 Protocol

**Location:** `src/codec/`

**Purpose:** Encode and decode all 16 MQTT 3.1.1 packet types.

### Key Types

```rust
// All packet types
pub enum MqttPacket {
    Connect(ConnectPacket),
    Connack(ConnackPacket),
    Publish(PublishPacket),
    Puback(PubackPacket),
    Pubrec(PubrecPacket),
    Pubrel(PubrelPacket),
    Pubcomp(PubcompPacket),
    Subscribe(SubscribePacket),
    Suback(SubackPacket),
    Unsubscribe(UnsubscribePacket),
    Unsuback(UnsubackPacket),
    Pingreq,
    Pingresp,
    Disconnect,
}

// Decoder
pub struct MqttDecoder;
impl MqttDecoder {
    pub fn decode(bytes: &mut BytesMut) -> Result<Option<MqttPacket>>;
}

// Encoder
pub struct MqttEncoder;
impl MqttEncoder {
    pub fn encode(packet: &MqttPacket, buf: &mut BytesMut) -> Result<()>;
}
```

### Files

| File | Description |
|------|-------------|
| `mod.rs` | Module re-exports |
| `packet.rs` | Packet type definitions |
| `types.rs` | Common types (QoS, ConnectFlags) |
| `decode.rs` | Packet decoding logic |
| `encode.rs` | Packet encoding logic |
| `varint.rs` | Variable-length integer codec |
| `error.rs` | Codec error types |
| `tests.rs` | Unit tests (60 tests) |

---

## `topic_matcher` — Wildcard Matching

**Location:** `src/topic_matcher/`

**Purpose:** Efficient topic filter matching with wildcard support.

### Key Types

```rust
// Parsed topic filter
pub struct TopicFilter {
    pub pattern: String,
    pub levels: Vec<Level>,
}

pub enum Level {
    Exact(String),
    SingleWildcard,  // +
    MultiWildcard,   // #
}

// Trie-based matcher
pub struct TopicTrie<T> {
    // Insert subscription
    pub fn insert(&mut self, filter: &TopicFilter, value: T);
    
    // Find all matching subscriptions
    pub fn matches(&self, topic: &str) -> Vec<&T>;
    
    // Remove subscription
    pub fn remove(&mut self, filter: &TopicFilter) -> Option<T>;
}
```

### Wildcard Rules

| Pattern | Matches | Doesn't Match |
|---------|---------|---------------|
| `sensors/+/temp` | `sensors/room1/temp` | `sensors/temp` |
| `sensors/#` | `sensors/a/b/c` | `other/topic` |
| `+/+/+` | `a/b/c` | `a/b` |

### Files

| File | Description |
|------|-------------|
| `mod.rs` | Module re-exports |
| `filter.rs` | TopicFilter parsing |
| `trie.rs` | Trie implementation |
| `validation.rs` | Topic/filter validation |
| `tests.rs` | Unit tests (23 tests) |

---

## `session` — Connection State

**Location:** `src/session/`

**Purpose:** Manage client sessions and QoS state machines.

### Key Types

```rust
// Client session
pub struct Session {
    pub client_id: String,
    pub clean_session: bool,
    pub subscriptions: HashMap<String, QoS>,
    pub connection_state: ConnectionState,
}

// QoS 1 state machine
pub enum Qos1State {
    AwaitingPuback { packet_id: u16, message: Message },
    Complete,
}

// QoS 2 state machine  
pub enum Qos2State {
    AwaitingPubrec { packet_id: u16, message: Message },
    AwaitingPubrel { packet_id: u16 },
    AwaitingPubcomp { packet_id: u16 },
    Complete,
}

// Packet ID allocator
pub struct PacketIdAllocator {
    pub fn allocate(&mut self) -> Option<u16>;
    pub fn release(&mut self, id: u16);
}
```

### Files

| File | Description |
|------|-------------|
| `mod.rs` | Module re-exports |
| `session.rs` | Session management |
| `connection.rs` | Connection state |
| `qos.rs` | QoS state machines |
| `packet_id.rs` | Packet ID allocation |
| `tests.rs` | Unit tests (24 tests) |

---

## `router` — Message Distribution

**Location:** `src/router/`

**Purpose:** Route published messages to matching subscribers.

### Key Types

```rust
// Central router
pub struct Router {
    subscriptions: TopicTrie<Subscriber>,
    retain_store: RetainStore,
    sessions: SessionStore,
}

impl Router {
    // Add subscription
    pub fn subscribe(&self, client_id: &str, filter: &str, qos: QoS);
    
    // Remove subscription
    pub fn unsubscribe(&self, client_id: &str, filter: &str);
    
    // Route message to subscribers
    pub fn route(&self, message: &PublishPacket) -> Vec<Delivery>;
    
    // Store retained message
    pub fn retain(&self, topic: &str, message: Option<Message>);
}

// Subscription registry
pub struct Registry {
    pub fn add(&self, filter: &str, subscriber: Subscriber);
    pub fn remove(&self, filter: &str, client_id: &str);
    pub fn matches(&self, topic: &str) -> Vec<Subscriber>;
}
```

### Files

| File | Description |
|------|-------------|
| `mod.rs` | Module re-exports |
| `router.rs` | Main router logic |
| `registry.rs` | Subscription registry |
| `retain.rs` | Retained message store |
| `tests.rs` | Unit tests (24 tests) |

---

## `persistence` — Storage Abstraction

**Location:** `src/persistence/`

**Purpose:** Abstract storage for sessions, messages, and retained data.

### Key Types

```rust
// Storage trait
#[async_trait]
pub trait Storage: Send + Sync {
    // Session operations
    async fn store_session(&self, session: &Session) -> Result<()>;
    async fn load_session(&self, client_id: &str) -> Result<Option<Session>>;
    async fn delete_session(&self, client_id: &str) -> Result<()>;
    
    // Message queue operations
    async fn enqueue(&self, client_id: &str, message: Message) -> Result<()>;
    async fn dequeue(&self, client_id: &str) -> Result<Vec<Message>>;
    
    // Retained message operations
    async fn store_retained(&self, topic: &str, message: Option<Message>) -> Result<()>;
    async fn load_retained(&self, topic: &str) -> Result<Option<Message>>;
}

// In-memory implementation
pub struct MemoryStore {
    sessions: RwLock<HashMap<String, Session>>,
    queues: RwLock<HashMap<String, VecDeque<Message>>>,
    retained: RwLock<HashMap<String, Message>>,
}
```

### Files

| File | Description |
|------|-------------|
| `mod.rs` | Module re-exports |
| `traits.rs` | Storage trait definitions |
| `memory.rs` | In-memory implementation |
| `tests.rs` | Unit tests (20 tests) |

---

## `transport` — Network Layer

**Location:** `src/transport/`

**Purpose:** TCP connection handling and MQTT framing.

### Key Types

```rust
// MQTT codec for tokio
pub struct MqttCodec;

impl Decoder for MqttCodec {
    type Item = MqttPacket;
    type Error = CodecError;
    
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>>;
}

impl Encoder<MqttPacket> for MqttCodec {
    type Error = CodecError;
    
    fn encode(&mut self, item: MqttPacket, dst: &mut BytesMut) -> Result<()>;
}

// Connection wrapper
pub struct Connection {
    inner: Framed<TcpStream, MqttCodec>,
}

impl Connection {
    pub async fn read(&mut self) -> Result<Option<MqttPacket>>;
    pub async fn write(&mut self, packet: MqttPacket) -> Result<()>;
}
```

### Files

| File | Description |
|------|-------------|
| `mod.rs` | Module re-exports |
| `codec.rs` | Tokio codec implementation |
| `connection.rs` | Connection wrapper |
| `tests.rs` | Unit tests (16 tests) |

---

## `server` — Broker Server

**Location:** `src/server/`

**Purpose:** Accept connections and handle MQTT protocol.

### Key Types

```rust
// Server configuration
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub max_connections: usize,
    pub max_packet_size: usize,
}

// Main server
pub struct MqttServer {
    config: ServerConfig,
    router: Arc<Router>,
}

impl MqttServer {
    pub async fn run(&self) -> Result<()>;
}

// Per-client handler
pub struct ClientHandler {
    connection: Connection,
    session: Option<Session>,
    router: Arc<Router>,
}

impl ClientHandler {
    pub async fn run(&mut self) -> Result<()>;
    
    async fn handle_connect(&mut self, packet: ConnectPacket) -> Result<()>;
    async fn handle_publish(&mut self, packet: PublishPacket) -> Result<()>;
    async fn handle_subscribe(&mut self, packet: SubscribePacket) -> Result<()>;
    // ... other packet handlers
}
```

### Files

| File | Description |
|------|-------------|
| `mod.rs` | Module re-exports |
| `server.rs` | Main server loop |
| `handler.rs` | Client handler |
| `client.rs` | Client state management |
| `tests.rs` | Integration tests (11 tests) |

---

## `bin/` — Executables

**Location:** `src/bin/`

**Purpose:** CLI entry points.

### Binaries

| Binary | Description |
|--------|-------------|
| `broker.rs` → `mqtt-broker` | Start the broker server |
| `producer.rs` → `mqtt-producer` | Publish messages |
| `consumer.rs` → `mqtt-consumer` | Subscribe and receive |

---

## Test Coverage

| Module | Tests | Coverage |
|--------|-------|----------|
| codec | 60 | Packet encode/decode roundtrips |
| topic_matcher | 23 | Wildcard matching, edge cases |
| session | 24 | State machines, packet IDs |
| router | 24 | Routing, subscriptions |
| persistence | 20 | Storage operations |
| transport | 16 | Framing, connection handling |
| server | 11 | Integration scenarios |
| **Total** | **178** | |

---

## Adding a New Module

1. Create directory: `src/mymodule/`
2. Add `mod.rs` with public exports
3. Add `tests.rs` with unit tests
4. Export from `src/lib.rs`: `pub mod mymodule;`
5. Add to this documentation

### Module Template

```rust
// src/mymodule/mod.rs
mod implementation;
mod tests;

pub use implementation::*;

// src/mymodule/implementation.rs
pub struct MyType {
    // ...
}

impl MyType {
    pub fn new() -> Self {
        // ...
    }
}

// src/mymodule/tests.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let my = MyType::new();
        // assertions
    }
}
```
