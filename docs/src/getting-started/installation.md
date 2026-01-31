# Installation

## Requirements

- **Rust 1.70+** — [Install Rust](https://rustup.rs/)
- **Git** — For cloning the repository

## From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/iliasichinava/mqtt-broker.git
cd mqtt-broker

# Build in release mode
cargo build --release

# The binaries are now in target/release/
ls target/release/mqtt-*
```

The following binaries will be built:

| Binary | Description |
|--------|-------------|
| `mqtt-broker` | The MQTT broker server |
| `mqtt-producer` | CLI tool to publish messages |
| `mqtt-consumer` | CLI tool to subscribe to topics |

## Install Globally

To install the binaries to your Cargo bin directory (`~/.cargo/bin/`):

```bash
cargo install --path .
```

Now you can run the commands from anywhere:

```bash
mqtt-broker --help
mqtt-producer --help
mqtt-consumer --help
```

## From GitHub Directly

Install directly from the repository without cloning:

```bash
cargo install --git https://github.com/iliasichinava/mqtt-broker.git
```

## Verify Installation

```bash
# Check broker version/help
mqtt-broker --help

# Expected output:
# MQTT Broker - Production-grade MQTT 3.1.1 broker
# 
# Usage: mqtt-broker [OPTIONS]
# ...
```

## Next Steps

Now that you have mqtt-broker installed, head to the [Quick Start](./quickstart.md) guide to run your first pub/sub workflow.
