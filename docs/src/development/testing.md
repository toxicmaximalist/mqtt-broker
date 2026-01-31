# Running Tests

Comprehensive testing is a core principle of this project. Here's how to run and write tests.

## Quick Start

Run all tests:

```bash
cargo test
```

Expected output:

```
running 178 tests
...
test result: ok. 178 passed; 0 failed; 0 ignored
```

## Test Commands

### Run All Tests

```bash
cargo test
```

### Run Tests with Output

See println! output from tests:

```bash
cargo test -- --nocapture
```

### Run Specific Test

```bash
# By name
cargo test test_publish_decode

# By partial name
cargo test publish

# By module
cargo test codec::
```

### Run Tests in Specific Module

```bash
# Codec tests only
cargo test --package mqtt-broker --lib codec::tests

# Topic matcher tests
cargo test --package mqtt-broker --lib topic_matcher::tests

# Session tests
cargo test --package mqtt-broker --lib session::tests
```

### Run Tests in Release Mode

```bash
cargo test --release
```

### Run Ignored Tests

Some tests are marked `#[ignore]` (slow or require setup):

```bash
cargo test -- --ignored
```

### Run All Including Ignored

```bash
cargo test -- --include-ignored
```

## Test Organization

```
src/
├── codec/
│   └── tests.rs          # 60 tests
├── topic_matcher/
│   └── tests.rs          # 23 tests
├── session/
│   └── tests.rs          # 24 tests
├── router/
│   └── tests.rs          # 24 tests
├── persistence/
│   └── tests.rs          # 20 tests
├── transport/
│   └── tests.rs          # 16 tests
└── server/
    └── tests.rs          # 11 tests

tests/
├── codec_proptest.rs     # Property-based tests
└── message_test.rs       # Integration tests
```

## Test Categories

### Unit Tests

Test individual functions and structs:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let packet = ConnectPacket {
            client_id: "test".to_string(),
            // ...
        };
        
        let mut buf = BytesMut::new();
        MqttEncoder::encode(&MqttPacket::Connect(packet.clone()), &mut buf).unwrap();
        
        let decoded = MqttDecoder::decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded, MqttPacket::Connect(packet));
    }
}
```

### Integration Tests

Test multiple components together (in `tests/` directory):

```rust
// tests/message_test.rs
use mqtt_broker::*;

#[tokio::test]
async fn test_publish_flow() {
    let router = Router::new();
    let session = Session::new("client-1", false);
    
    // Subscribe
    router.subscribe("client-1", "sensors/#", QoS::AtLeastOnce);
    
    // Publish
    let message = PublishPacket {
        topic: "sensors/temp".to_string(),
        payload: b"25".to_vec(),
        // ...
    };
    
    let deliveries = router.route(&message);
    assert_eq!(deliveries.len(), 1);
}
```

### Property-Based Tests

Using `proptest` for randomized testing:

```rust
// tests/codec_proptest.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_varint_roundtrip(value in 0u32..268435455) {
        let mut buf = BytesMut::new();
        encode_variable_int(value, &mut buf).unwrap();
        let decoded = decode_variable_int(&mut buf).unwrap();
        prop_assert_eq!(value, decoded);
    }
}
```

Run property tests:

```bash
cargo test proptest
```

### Async Tests

Using `#[tokio::test]` for async code:

```rust
#[tokio::test]
async fn test_server_accept() {
    let server = MqttServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();
    
    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    let client = TcpStream::connect(addr).await.unwrap();
    assert!(client.peer_addr().is_ok());
}
```

## Test Coverage

### Using cargo-tarpaulin

```bash
# Install
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html

# Open report
open tarpaulin-report.html
```

### Using llvm-cov

```bash
# Install
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov

# Run coverage
cargo llvm-cov --html

# Open report
open target/llvm-cov/html/index.html
```

## Benchmarks

### Criterion Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench publish
```

Benchmark location: `benches/`

Example benchmark:

```rust
// benches/codec_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};
use mqtt_broker::codec::*;

fn bench_publish_encode(c: &mut Criterion) {
    let packet = PublishPacket {
        topic: "sensors/temperature".to_string(),
        payload: vec![0u8; 1024],
        // ...
    };
    
    c.bench_function("publish_encode_1kb", |b| {
        b.iter(|| {
            let mut buf = BytesMut::with_capacity(2048);
            MqttEncoder::encode(&MqttPacket::Publish(packet.clone()), &mut buf)
        })
    });
}

criterion_group!(benches, bench_publish_encode);
criterion_main!(benches);
```

## Writing New Tests

### Test Naming Convention

```rust
#[test]
fn test_<module>_<function>_<scenario>() {
    // ...
}

// Examples:
fn test_codec_publish_empty_payload() { }
fn test_topic_filter_multi_wildcard() { }
fn test_session_clean_session_true() { }
```

### Test Structure (AAA Pattern)

```rust
#[test]
fn test_subscribe_adds_to_trie() {
    // Arrange
    let mut router = Router::new();
    
    // Act
    router.subscribe("client-1", "sensors/#", QoS::AtLeastOnce);
    
    // Assert
    let matches = router.matches("sensors/temp");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].client_id, "client-1");
}
```

### Testing Error Cases

```rust
#[test]
fn test_invalid_topic_filter_returns_error() {
    let result = TopicFilter::parse("sensors/#/invalid");
    assert!(result.is_err());
    assert!(matches!(result, Err(TopicError::InvalidFilter(_))));
}
```

### Testing Async Code

```rust
#[tokio::test]
async fn test_connection_timeout() {
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        Connection::connect("127.0.0.1:9999")
    ).await;
    
    assert!(result.is_err()); // Timeout
}
```

## CI Integration

Tests run automatically on GitHub Actions:

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features
```

## Troubleshooting

### Tests Hang

Usually a deadlock or missing `.await`:

```bash
# Run with timeout
timeout 60 cargo test

# Run single-threaded
cargo test -- --test-threads=1
```

### Flaky Tests

Add retry or increase timeouts:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_publishes() {
    // Use tokio::time::timeout
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        async_operation()
    ).await;
    
    assert!(result.is_ok());
}
```

### Port Already in Use

Tests using network ports can conflict:

```rust
// Use port 0 for automatic assignment
let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
let addr = listener.local_addr().unwrap();
```

### Debug Failing Tests

```bash
# Verbose output
cargo test test_name -- --nocapture

# With RUST_BACKTRACE
RUST_BACKTRACE=1 cargo test test_name

# With logging
RUST_LOG=debug cargo test test_name -- --nocapture
```
