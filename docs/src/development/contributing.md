# Contributing

Thank you for your interest in contributing to this MQTT broker project!

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork:**
   ```bash
   git clone https://github.com/yourusername/mqtt-broker.git
   cd mqtt-broker
   ```
3. **Set upstream remote:**
   ```bash
   git remote add upstream https://github.com/originalowner/mqtt-broker.git
   ```
4. **Create a feature branch:**
   ```bash
   git checkout -b feature/my-feature
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Git
- (Optional) Docker for integration testing

### Build and Test

```bash
# Build
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --bin mqtt-broker
```

### Useful Commands

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check without building
cargo check

# Update dependencies
cargo update

# Generate docs
cargo doc --open
```

## Code Style

### Formatting

Run `cargo fmt` before committing:

```bash
cargo fmt
```

We use the default rustfmt configuration.

### Linting

All code must pass `cargo clippy`:

```bash
cargo clippy -- -D warnings
```

Fix warnings before submitting PRs.

### Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Structs | PascalCase | `TopicFilter` |
| Traits | PascalCase | `Storage` |
| Functions | snake_case | `encode_packet` |
| Variables | snake_case | `client_id` |
| Constants | SCREAMING_SNAKE | `MAX_PACKET_SIZE` |
| Modules | snake_case | `topic_matcher` |

### Documentation

All public items should have documentation:

```rust
/// Encodes an MQTT packet to bytes.
///
/// # Arguments
///
/// * `packet` - The packet to encode
/// * `buf` - Buffer to write encoded bytes to
///
/// # Returns
///
/// Returns `Ok(())` on success, or `CodecError` on failure.
///
/// # Example
///
/// ```rust
/// let packet = MqttPacket::Pingreq;
/// let mut buf = BytesMut::new();
/// MqttEncoder::encode(&packet, &mut buf)?;
/// ```
pub fn encode(packet: &MqttPacket, buf: &mut BytesMut) -> Result<(), CodecError> {
    // ...
}
```

## Making Changes

### Types of Contributions

- 🐛 **Bug fixes** — Fix issues with tests
- ✨ **Features** — Add new functionality
- 📚 **Documentation** — Improve docs, examples
- 🧪 **Tests** — Add missing tests
- ⚡ **Performance** — Optimize code
- 🔧 **Refactoring** — Improve code structure

### Branch Naming

```
feature/description    # New features
fix/description        # Bug fixes
docs/description       # Documentation
test/description       # Test additions
refactor/description   # Code refactoring
```

Examples:
- `feature/qos2-support`
- `fix/packet-decode-crash`
- `docs/session-examples`

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `test`: Tests
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `chore`: Maintenance

Examples:

```
feat(codec): add MQTT 5 property parsing

fix(session): prevent double-free on disconnect

docs(readme): add installation instructions

test(router): add wildcard matching edge cases
```

## Pull Request Process

### Before Submitting

1. **Update your branch:**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all checks:**
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```

3. **Add tests** for new functionality

4. **Update documentation** if needed

### PR Guidelines

- **One feature per PR** — Keep PRs focused
- **Descriptive title** — What does this PR do?
- **Link issues** — Reference related issues
- **Include tests** — All new code needs tests
- **Update CHANGELOG** — Add entry under "Unreleased"

### PR Template

```markdown
## Description

Brief description of the changes.

## Related Issues

Closes #123

## Changes

- Added X
- Fixed Y
- Updated Z

## Testing

- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist

- [ ] Code formatted with `cargo fmt`
- [ ] No clippy warnings
- [ ] Documentation updated
- [ ] CHANGELOG updated
```

## Testing Requirements

### All PRs Must

1. **Pass existing tests** — No regressions
2. **Add new tests** — For new functionality
3. **Maintain coverage** — Don't decrease coverage

### Test Guidelines

```rust
// Good: Descriptive name, tests one thing
#[test]
fn test_publish_with_empty_payload_succeeds() {
    let packet = PublishPacket {
        payload: vec![],
        // ...
    };
    assert!(encode(&packet).is_ok());
}

// Good: Tests error case
#[test]
fn test_invalid_topic_returns_error() {
    let result = TopicFilter::parse("invalid/#/topic");
    assert!(matches!(result, Err(TopicError::InvalidFilter(_))));
}
```

## Module Structure

When adding new modules:

```
src/mymodule/
├── mod.rs          # Public exports
├── types.rs        # Type definitions
├── implementation.rs
└── tests.rs        # Unit tests
```

Export from `src/lib.rs`:

```rust
pub mod mymodule;
```

## Error Handling

Use the project's error types:

```rust
// Good: Use existing error types
fn parse_topic(s: &str) -> Result<Topic, TopicError> {
    // ...
}

// Good: Add new variant if needed
#[derive(Debug, Error)]
pub enum TopicError {
    #[error("invalid topic: {0}")]
    InvalidTopic(String),
    
    #[error("topic too long: {length} > {max}")]
    TooLong { length: usize, max: usize },
}
```

## Getting Help

- **Questions?** Open a discussion on GitHub
- **Bug found?** Open an issue with reproduction steps
- **Feature idea?** Open an issue to discuss first

## Code of Conduct

- Be respectful and constructive
- Focus on the code, not the person
- Assume good intentions
- Help others learn

## Recognition

Contributors are recognized in:
- CHANGELOG.md (for each release)
- GitHub contributors page

Thank you for contributing! 🎉
