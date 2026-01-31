# Contributing to mqtt-broker

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. We're all here to learn and build something useful together.

## How to Contribute

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Use the bug report template
3. Include:
   - Rust version (`rustc --version`)
   - OS and version
   - Steps to reproduce
   - Expected vs actual behavior
   - Relevant logs or error messages

### Suggesting Features

1. Check existing issues and discussions
2. Describe the use case
3. Explain why existing features don't meet the need
4. Propose a solution if you have one

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test`)
6. Run clippy (`cargo clippy`)
7. Format code (`cargo fmt`)
8. Commit with clear messages
9. Push and open a PR

## Development Setup

```bash
# Clone the repo
git clone https://github.com/iliasichinava/mqtt-broker.git
cd mqtt-broker

# Build
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features

# Format
cargo fmt
```

## Project Structure

```
src/
├── codec/          # MQTT packet encoding/decoding
├── topic_matcher/  # Topic wildcard matching
├── session/        # Session and QoS state
├── router/         # Message routing
├── persistence/    # Storage layer
├── transport/      # TCP handling
├── server/         # Broker server
├── bin/            # CLI binaries
└── lib.rs          # Library root
tests/              # Integration tests
```

## Coding Guidelines

### Style

- Follow Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for lints
- Write documentation for public APIs
- Keep functions focused and small

### Testing

- Write unit tests for new functionality
- Add integration tests for complex features
- Use property-based tests where appropriate
- Aim for meaningful coverage, not 100%

### Documentation

- Add doc comments (`///`) to public items
- Include examples in documentation
- Update README if adding features
- Update CHANGELOG for notable changes

### Commits

- Use clear, descriptive commit messages
- Follow conventional commits format:
  - `feat:` new features
  - `fix:` bug fixes
  - `docs:` documentation
  - `test:` test changes
  - `refactor:` code refactoring
  - `perf:` performance improvements

## Areas for Contribution

### Good First Issues

- Documentation improvements
- Additional test cases
- Code cleanup and refactoring
- Error message improvements

### Intermediate

- New persistence backends
- Performance optimizations
- Additional CLI features
- Logging improvements

### Advanced

- TLS/SSL support
- WebSocket transport
- MQTT 5.0 features
- Clustering support

## Questions?

Feel free to open an issue for questions or join discussions. We're happy to help!

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
