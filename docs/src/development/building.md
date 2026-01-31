# Building from Source

Build the MQTT broker and CLI tools from source.

## Prerequisites

### Rust Toolchain

Install Rust via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Minimum version: **Rust 1.70+** (uses edition 2021)

Verify installation:

```bash
rustc --version
cargo --version
```

### Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux (x86_64) | ✅ | Primary development |
| macOS (x86_64) | ✅ | Fully supported |
| macOS (ARM64) | ✅ | Apple Silicon |
| Windows (x86_64) | ✅ | MSVC toolchain |

## Clone Repository

```bash
git clone https://github.com/yourusername/mqtt-broker.git
cd mqtt-broker
```

## Build Commands

### Debug Build

Fast compilation, includes debug symbols, no optimizations:

```bash
cargo build
```

Binaries are placed in `target/debug/`:

```
target/debug/
├── mqtt-broker
├── mqtt-consumer
└── mqtt-producer
```

### Release Build

Optimized for production, slower to compile:

```bash
cargo build --release
```

Binaries are placed in `target/release/`:

```
target/release/
├── mqtt-broker
├── mqtt-consumer
└── mqtt-producer
```

### Build Specific Binary

```bash
# Only the broker
cargo build --bin mqtt-broker --release

# Only the producer
cargo build --bin mqtt-producer --release

# Only the consumer
cargo build --bin mqtt-consumer --release
```

### Build Library Only

```bash
cargo build --lib
```

## Install Locally

Install to `~/.cargo/bin/` (should be in your PATH):

```bash
cargo install --path .
```

Now available anywhere:

```bash
mqtt-broker --help
mqtt-producer --help
mqtt-consumer --help
```

## Build Configuration

### Cargo.toml Features

Currently no optional features. All functionality is included by default.

### Profile Settings

Release profile in `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

This produces smaller, faster binaries but increases compile time.

### Reducing Binary Size

For minimal binaries:

```bash
# Strip debug symbols
cargo build --release
strip target/release/mqtt-broker

# Or use build flags
RUSTFLAGS="-C link-arg=-s" cargo build --release
```

## Cross Compilation

### Install Target

```bash
# Linux on macOS
rustup target add x86_64-unknown-linux-gnu

# Windows on Linux/macOS
rustup target add x86_64-pc-windows-gnu

# ARM64 Linux
rustup target add aarch64-unknown-linux-gnu
```

### Cross Build

```bash
# Linux
cargo build --release --target x86_64-unknown-linux-gnu

# Windows
cargo build --release --target x86_64-pc-windows-gnu
```

Note: Cross compilation may require additional linkers. Consider using [cross](https://github.com/cross-rs/cross):

```bash
cargo install cross
cross build --release --target x86_64-unknown-linux-gnu
```

## Docker Build

### Build Image

```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /usr/src/mqtt-broker
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /usr/src/mqtt-broker/target/release/mqtt-broker /usr/local/bin/
EXPOSE 1883
CMD ["mqtt-broker"]
```

Build and run:

```bash
docker build -t mqtt-broker .
docker run -p 1883:1883 mqtt-broker
```

### Multi-Architecture

```bash
docker buildx build --platform linux/amd64,linux/arm64 -t mqtt-broker .
```

## Documentation Build

### Generate API Docs

```bash
cargo doc --open
```

Opens documentation in your browser.

### Build mdBook Docs

```bash
# Install mdBook
cargo install mdbook

# Build documentation site
cd docs
mdbook build

# Serve locally
mdbook serve --open
```

## Troubleshooting

### Compilation Errors

**Missing dependencies:**

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install build-essential

# macOS
xcode-select --install
```

**Outdated Rust:**

```bash
rustup update
```

### Linker Errors

**macOS:**

```bash
xcode-select --install
```

**Linux:**

```bash
sudo apt-get install gcc
```

### Slow Builds

Use incremental compilation (enabled by default) and consider:

```bash
# Use mold linker (Linux)
sudo apt-get install mold
RUSTFLAGS="-C link-arg=-fuse-ld=mold" cargo build

# Use lld linker
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build
```

### Out of Memory

Reduce parallelism:

```bash
cargo build -j 2
```

## Build Artifacts

After a successful build:

```
target/
├── debug/
│   ├── mqtt-broker        # Debug binary
│   ├── mqtt-consumer
│   ├── mqtt-producer
│   ├── deps/              # Dependencies
│   └── incremental/       # Incremental build cache
├── release/
│   ├── mqtt-broker        # Release binary
│   ├── mqtt-consumer
│   └── mqtt-producer
└── doc/                   # Generated documentation
    └── mqtt_broker/
```

## Clean Build

Remove all build artifacts:

```bash
cargo clean
```

Remove only release artifacts:

```bash
cargo clean --release
```
