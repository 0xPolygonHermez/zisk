# Consensus Network - Decentralized P2P Microservice

A production-ready Rust microservice designed for decentralized peer-to-peer task coordination with secure networking capabilities.

## Architecture

This project is organized as a Rust workspace with multiple crates:

```
crates/
├── consensus-server/     # Main binary orchestrating the application
├── consensus-core/       # Core business logic and shared functionality
├── consensus-api/        # HTTP API layer with REST endpoints
├── consensus-comm/       # P2P communication layer
├── consensus-config/     # Configuration management
└── consensus-client/     # Command-line client
```

## Command Line Client

The project includes a powerful CLI client for interacting with the server:

```bash
# Build the client
cargo build --bin consensus-client

# Basic usage
consensus-client --url http://localhost:3030 status
consensus-client --url http://localhost:3030 health
consensus-client --url http://localhost:3030 task --task-type "compute" --payload '{"data": "value"}' --priority 5

# Interactive mode
consensus-client --url http://localhost:3030 interactive
```

## Configuration

Configuration is loaded from multiple sources with the following priority:
1. Environment variables (with `CONSENSUS_` prefix)
2. Configuration files (`config/local.toml`, `config/default.toml`)
3. Built-in defaults

### Environment Variables

```bash
# Service Configuration
CONSENSUS_SERVICE_NAME="consensus-network"
CONSENSUS_SERVICE_ENVIRONMENT="production"

# Server Configuration
CONSENSUS_SERVER_HOST="0.0.0.0"
CONSENSUS_SERVER_PORT=8080

# Logging Configuration
CONSENSUS_LOGGING_LEVEL="info"
CONSENSUS_LOGGING_FORMAT="json"  # json, pretty, compact
CONSENSUS_LOGGING_FILE_OUTPUT=false

# Communication Configuration
CONSENSUS_COMM_MAX_PEERS=100
CONSENSUS_COMM_DISCOVERY_INTERVAL_SECONDS=30
CONSENSUS_COMM_HEARTBEAT_INTERVAL_SECONDS=10
```

## Development

### Prerequisites

- Rust 1.70+
- Cargo

### Running the Service

```bash
# Development mode (server with pretty logging)
cargo run --bin consensus-server

# Production mode with JSON logging
CONSENSUS_LOGGING_FORMAT=json CONSENSUS_LOGGING_LEVEL=info cargo run --bin consensus-server --release

# With custom port to avoid conflicts
CONSENSUS_SERVER_PORT=3030 cargo run --bin consensus-server

# Run client (connects to server)
cargo run --bin consensus-client -- --url http://localhost:3030 status
```

### Client CLI

```bash
# Check server health
consensus-client --url http://localhost:3030 health

# Submit a computation task
consensus-client --url http://localhost:3030 task \
  --task-type "computation" \
  --payload '{"algorithm": "factorial", "input": 10}' \
  --priority 5

# Interactive mode for multiple commands
consensus-client --url http://localhost:3030 interactive
```

## Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/consensus-network /usr/local/bin/
EXPOSE 8080
CMD ["consensus-network"]
```

## License

MIT OR Apache-2.0
