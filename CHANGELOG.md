# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- TLS/SSL support
- WebSocket transport
- MQTT 5.0 protocol support
- Clustering support
- Metrics and monitoring

## [0.1.0] - 2026-01-31

### Added
- Initial release
- Full MQTT 3.1.1 protocol support
- All 14 packet types (CONNECT, CONNACK, PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBSCRIBE, SUBACK, UNSUBSCRIBE, UNSUBACK, PINGREQ, PINGRESP, DISCONNECT)
- QoS 0, 1, and 2 delivery guarantees
- Topic wildcard matching (`+` and `#`)
- Retained message support
- Last Will and Testament (LWT)
- Clean session handling
- Session persistence
- Keep-alive timeout detection
- Async/await architecture with Tokio
- CLI tools: `mqtt-broker`, `mqtt-producer`, `mqtt-consumer`
- Comprehensive test suite (176+ tests)
- Property-based testing with proptest

### Architecture
- `codec` - Wire protocol encoding/decoding
- `topic_matcher` - Trie-based O(k) topic matching
- `session` - Connection and QoS state machines
- `router` - Message routing and subscriptions
- `persistence` - Pluggable storage layer
- `transport` - TCP framing and connections
- `server` - Full broker implementation

[Unreleased]: https://github.com/iliasichinava/mqtt-broker/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/iliasichinava/mqtt-broker/releases/tag/v0.1.0
