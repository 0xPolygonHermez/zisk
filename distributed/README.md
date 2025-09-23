# ZisK Distributed Proving System

A distributed proof generation system for the ZisK zkVM, consisting of a coordinator service that orchestrates proof tasks and multiple worker nodes that generate proofs. The system enables horizontal scaling of proof generation workloads across multiple machines.

## Architecture

The distributed system consists of several components organized as a Rust workspace:

```
distributed/
├── crates/
│   ├── coordinator/          # Coordinator service managing proof jobs and task distribution
│   ├── worker/              # Worker nodes that execute proof generation tasks
│   ├── grpc-api/            # gRPC API definitions and protocol (see grpc-api/README.md for details)
│   └── common/              # Shared types and utilities
├── docker-compose.yml       # Docker deployment configuration
├── Dockerfile              # Container build configuration
└── README.md               # This file
```

### Components

- **Coordinator**: Central service that receives proof requests, manages job queues, and distributes tasks to available workers
- **Workers**: Compute nodes that connect to the coordinator and execute assigned proof generation tasks
- **gRPC API**: Communication protocol between coordinator and workers (bidirectional streaming)

## Quick Start

### Docker Deployment (Recommended)

The easiest way to run the distributed system:

```bash
# From the distributed directory
cd distributed/

# Build and start services (1 coordinator + 1 worker)
docker-compose up -d

# View logs
docker-compose logs -f coordinator
docker-compose logs -f worker

# Scale to multiple workers
docker-compose up -d --scale worker=4

# Stop services
docker-compose down
```

### Manual Build and Run

```bash
# Build binaries (from project root)
cargo build --release --bin zisk-coordinator --bin zisk-worker

# Run coordinator
cargo run --bin zisk-coordinator -- server --port 8080

# Run worker (in another terminal)
cargo run --bin zisk-worker -- --url http://127.0.0.1:8080
```

## Configuration

The system supports flexible configuration through TOML files and environment variables.

### Coordinator Configuration

#### Environment Variables

```bash
# Server Configuration
DISTRIBUTED_SERVER_HOST="0.0.0.0"          # Bind address
DISTRIBUTED_SERVER_PORT=8080                # gRPC port (Docker uses 50051)

# Service Configuration  
DISTRIBUTED_SERVICE_NAME="zisk-distributed"
DISTRIBUTED_SERVICE_ENVIRONMENT="production"

# Logging Configuration
DISTRIBUTED_LOGGING_LEVEL="info"            # trace, debug, info, warn, error
DISTRIBUTED_LOGGING_FORMAT="json"           # json, pretty, compact
DISTRIBUTED_LOGGING_FILE_OUTPUT=true
DISTRIBUTED_LOGGING_FILE_PATH="/var/log/distributed/coordinator.log"
```

#### Command Line Arguments

```bash
# Show help
cargo run --bin zisk-coordinator -- --help

# Run with custom port
cargo run --bin zisk-coordinator -- server --port 50051

# Run with specific configuration
cargo run --bin zisk-coordinator -- --config /path/to/coordinator-config.toml
```

### Worker Configuration

#### Configuration File Generation

Workers support rich configuration through TOML files:

```bash
# Generate example configuration file
cargo run --bin zisk-worker -- --generate-config
```

This creates a `config.toml` file with documented options:

```toml
[server]
# URL of the coordinator to connect to
url = "http://127.0.0.1:8080"

[prover]
# Prover ID (optional - auto-generated UUID if not set)
# prover_id = "worker-001"

[prover.compute_capacity]
# Number of compute units this worker can handle
compute_units = 4

[connection]
# Reconnection interval after disconnect (seconds)
reconnect_interval_seconds = 5
# Heartbeat timeout (seconds) 
heartbeat_timeout_seconds = 30
```

#### Environment Variables

```bash
# Server Connection
DISTRIBUTED_SERVER_URL="http://coordinator:50051"

# Worker Identity  
DISTRIBUTED_PROVER_ID="worker-001"

# Logging Configuration
DISTRIBUTED_LOGGING_LEVEL="info"
DISTRIBUTED_LOGGING_FORMAT="json"
DISTRIBUTED_LOGGING_FILE_PATH="/var/log/distributed/worker.log"
```

#### Command Line Arguments

```bash
# Show help and available options
cargo run --bin zisk-worker -- --help

# Override server URL
cargo run --bin zisk-worker -- --url "http://production-coordinator:8080"

# Override prover ID  
cargo run --bin zisk-worker -- --prover-id "production-worker-001"

# Override compute capacity
cargo run --bin zisk-worker -- --compute-units 16

# Use custom configuration file
cargo run --bin zisk-worker -- --config production-worker.toml

# Combine multiple overrides
cargo run --bin zisk-worker -- \
  --url "http://production-coordinator:8080" \
  --compute-units 32 \
  --prover-id "high-capacity-worker"
```

#### Configuration Priority

Configuration values are resolved in this priority order:

1. **Command line arguments** (highest priority)
2. **Configuration file** (specified by `--config` or default `config.toml`)
3. **Environment variables** (with `DISTRIBUTED_` prefix)
4. **Built-in defaults** (lowest priority)

#### Worker Configuration Reference

| Configuration Key | CLI Argument | Environment Variable | Type | Default | Description |
|------------------|--------------|---------------------|------|---------|-------------|
| `server.url` | `--url` | `DISTRIBUTED_SERVER_URL` | String | `http://127.0.0.1:8080` | Coordinator server URL |
| `prover.prover_id` | `--prover-id` | `DISTRIBUTED_PROVER_ID` | String | Auto-generated UUID | Unique worker identifier |
| `prover.compute_capacity.compute_units` | `--compute-units` | - | Number | CPU count | Available compute units |
| `connection.reconnect_interval_seconds` | - | - | Number | 5 | Reconnection interval in seconds |
| `connection.heartbeat_timeout_seconds` | - | - | Number | 30 | Heartbeat timeout in seconds |

#### Alternative Configuration File Path

You can also specify the configuration file path using an environment variable:

```bash
export CONFIG_PATH="/path/to/my-config.toml"
cargo run --bin zisk-worker
```

## Deployment Scenarios

### Development Setup

Quick local testing with default settings:

```bash
# Terminal 1: Start coordinator
cargo run --bin zisk-coordinator

# Terminal 2: Start worker
cargo run --bin zisk-worker

# Terminal 3: Start additional worker with different ID
cargo run --bin zisk-worker -- --prover-id "dev-worker-2" --compute-units 2
```

### Production Docker Deployment

```bash
# Create production configuration
mkdir -p /etc/zisk-distributed
cp distributed/docker-compose.yml /etc/zisk-distributed/
cd /etc/zisk-distributed

# Edit docker-compose.yml for production settings
# - Change ports as needed
# - Update logging configuration  
# - Set appropriate compute_units for workers
# - Configure persistent volumes for logs

# Deploy
docker-compose up -d --scale worker=8

# Monitor
docker-compose logs -f
docker-compose ps
```

### Multi-Machine Deployment

For distributed deployment across multiple machines:

1. **Coordinator machine**:
   ```bash
   # Run coordinator with external binding
   docker run -p 50051:50051 \
     -e DISTRIBUTED_SERVER_HOST=0.0.0.0 \
     -e DISTRIBUTED_SERVER_PORT=50051 \
     zisk-distributed coordinator
   ```

2. **Worker machines**:
   ```bash
   # Point workers to coordinator IP
   docker run \
     -e DISTRIBUTED_SERVER_URL=http://COORDINATOR_IP:50051 \
     -e DISTRIBUTED_PROVER_ID=worker-machine-1 \
     -e DISTRIBUTED_LOGGING_LEVEL=info \
     zisk-distributed worker
   ```

## Administrative Operations

### Health Checks and Monitoring

The coordinator exposes administrative endpoints for monitoring:

```bash
# Basic health check
grpcurl -plaintext 127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/HealthCheck

# System status
grpcurl -plaintext 127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/SystemStatus

# List active jobs
grpcurl -plaintext -d '{"active_only": true}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/JobsList

# List connected workers
grpcurl -plaintext -d '{"available_only": true}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/ProversList
```

### Scaling Operations

```bash
# Scale up workers in Docker
docker-compose up -d --scale worker=10

# Scale down workers  
docker-compose up -d --scale worker=2

# Add worker on different machine
docker run -d \
  -e DISTRIBUTED_SERVER_URL=http://coordinator-host:50051 \
  -e DISTRIBUTED_PROVER_ID=remote-worker-1 \
  zisk-distributed worker
```

## Troubleshooting

### Common Issues

**Worker can't connect to coordinator:**
- Verify coordinator is running and accessible on the specified port
- Check firewall settings if coordinator and worker are on different machines
- Ensure correct URL format: `http://host:port` (not `https://` for default setup)

**Configuration not loading:**
- Use `--generate-config` to create a valid example configuration
- Verify TOML syntax with a TOML validator
- Check file permissions on configuration files
- Use CLI overrides to test specific values

**Worker not receiving tasks:**
- Check worker registration in coordinator logs
- Verify compute capacity is appropriate for available tasks
- Ensure worker ID is unique if running multiple workers
- Confirm coordinator has active jobs to distribute

**Port conflicts:**
- Coordinator defaults to port 8080, Docker uses 50051
- Use `--port` flag or `DISTRIBUTED_SERVER_PORT` environment variable to change
- Check for other services using the same ports

### Debug Mode

Enable detailed logging for troubleshooting:

```bash
# Coordinator with debug logging
DISTRIBUTED_LOGGING_LEVEL=debug DISTRIBUTED_LOGGING_FORMAT=pretty \
  cargo run --bin zisk-coordinator

# Worker with debug logging
DISTRIBUTED_LOGGING_LEVEL=debug DISTRIBUTED_LOGGING_FORMAT=pretty \
  cargo run --bin zisk-worker -- --url http://127.0.0.1:8080
```

### Testing Configuration

Use CLI overrides to test specific values without modifying configuration files:

```bash
# Test connection to different coordinator
cargo run --bin zisk-worker -- --url http://test-coordinator:8080

# Test with specific capacity and ID
cargo run --bin zisk-worker -- --compute-units 8 --prover-id test-worker

# Test coordinator with different port
cargo run --bin zisk-coordinator -- server --port 9090
```

### Log Files

When file logging is enabled, logs are written to:
- Coordinator: `/var/log/distributed/coordinator.log`
- Worker: `/var/log/distributed/worker.log`

In Docker deployments, logs are accessible via:
```bash
docker-compose logs coordinator
docker-compose logs worker
```

## Development

### Prerequisites

- Rust 1.75+ (see `rust-toolchain.toml`)
- Protocol Buffers compiler (`protoc`)
- Docker & Docker Compose (optional, for containerized development)

### Building from Source

```bash
# From workspace root
cargo build --release

# Build only distributed components
cargo build --release --bin zisk-coordinator --bin zisk-worker

# Run tests
cargo test -p zisk-distributed-coordinator -p zisk-distributed-worker

# Check formatting
cargo fmt --check

# Run lints
cargo clippy -- -D warnings
```

### gRPC API Development

For details on the gRPC protocol, message types, and API development, see [`crates/grpc-api/README.md`](crates/grpc-api/README.md).
