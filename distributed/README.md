# ZisK Distributed Proving System

A distributed proof generation system for the ZisK zkVM that orchestrates proof tasks across multiple worker nodes. The system enables horizontal scaling of proof generation workloads, allowing you to distribute computationally intensive proving operations across multiple machines for improved performance and throughput.

The system is composed of two main actors:

- **Coordinator:** Manages incoming proof requests and splits the work, based on required compute capacity, across distributed available workers.  
- **Worker:** Registers to the coordinator, reporting its compute capacity, and waits for tasks to be assigned. A **Worker** can be a single machine or a cluster of machines.

The process of generating a proof proceeds as follows:  
1. The **Coordinator** starts on a host and listens for incoming proof requests.  
2. **Worker** nodes connect to the Coordinator, registering their compute capacity and availability.  
3. When a proof generation request is received, the Coordinator splits the work across multiple Workers according to the requested compute capacity. A job is divided into three phases:
   - **Partial Contributions:** Worker computes its partial challenges.
   - **Prove:** Workers compute the global challenge and their respective partial proofs.  
   - **Aggregation:** A Worker aggregates the partial proofs and produces the final proof for the client.
4. The Coordinator collects the final proof and returns it to the client.

## Quick Start

### Manual Build and Run

```bash
# Build binaries (from project root)
cargo build --release --bin zisk-coordinator --bin zisk-worker

# Run coordinator
cargo run --release --bin zisk-coordinator -- --port 50051

# Run a worker node (in another terminal)
cargo run --release --bin zisk-worker -- --coordinator-url http://127.0.0.1:50051 --witness-lib <path-to-libzisk_witness.so> --proving-key <path-to-provingKey> --elf <path-to-elf-file> --asm-port <port-number>

# Generate a proof (in another terminal)
cargo run --release --bin zisk-coordinator prove-block --coordinator-url http://127.0.0.1:50051 --input <path-to-inputs> --compute-capacity 10
```

### Docker Deployment

The easiest way to run the distributed system:

```bash
# From the distributed directory
cd distributed/

# 0. Build the Docker image (CPU-only, default)
docker build -t zisk-distributed:latest -f Dockerfile ..

# 0b. Build with GPU support (if needed)
docker build --build-arg GPU=true -t zisk-distributed:gpu -f Dockerfile ..

# Create a user-defined network so container names resolve via DNS
docker network create zisk-net || true

# 1. Start coordinator container (detached)
LOGS_DIR="$(pwd)/../logs"
docker run --rm --name zisk-coordinator \
  --network zisk-net -p 50051:50051 \
  -v "$LOGS_DIR:/var/log/distributed" \
  -e RUST_LOG=info \
  zisk-distributed:latest \
  zisk-coordinator --config /app/config/coordinator/dev.toml

# 2. Start worker container(s) - they connect to coordinator by container name
# Replace paths with your actual directories
LOGS_DIR="$(pwd)/../logs"
PROVING_KEY_DIR="$(pwd)/../build/provingKey"
ELF_DIR="$(pwd)/../../zisk-testvectors/eth-client/elf"
INPUTS_DIR="$(pwd)/../../zisk-testvectors/eth-client/inputs"
docker run --rm --name zisk-worker-1 \
  --network zisk-net --shm-size=20g \
  -v "$LOGS_DIR:/var/log/distributed" \
  -v "$HOME/.zisk/cache:/app/.zisk/cache:ro" \
  -v "$PROVING_KEY_DIR:/app/proving-keys:ro" \
  -v "$ELF_DIR:/app/elf:ro" \
  -v "$INPUTS_DIR:/app/inputs:ro" \
  -e RUST_LOG=info \
  zisk-distributed:latest zisk-worker \
    --config /app/config/worker/dev.toml --coordinator-url http://zisk-coordinator:50051 \
    --elf /app/elf/zec.elf --proving-key /app/proving-keys --asm-port 15200

# View logs
docker logs -f zisk-coordinator
docker logs -f zisk-worker-1

# Generate a proof inside the coordinator container
docker exec -it zisk-coordinator \
  zisk-coordinator prove-block --coordinator-url http://127.0.0.1:50051 \
  --input /app/inputs/21429992_1_0.bin --compute-capacity 10

# Stop containers
docker stop zisk-coordinator zisk-worker-1
docker rm zisk-coordinator zisk-worker-1
```

**Note:** 
- **GPU Support:** Use `--build-arg GPU=true` when building if you need GPU acceleration
- **Configuration:** Built-in configs are used by default, no external mounting needed
- **Paths in container:**
  - Configuration: `/app/config/{coordinator,worker}/`
  - Binaries: `/app/bin/`
  - Cache: `/app/.zisk/cache/` (mounted from host `$HOME/.zisk/cache`)
  - Logs: `/var/log/distributed/`

## Configuration

The system supports flexible configuration through TOML files and environment variables.

### Coordinator Configuration

The coordinator can be configured through TOML configuration files and command line arguments.

#### Configuration Files

Example development configuration (`development.toml`):
```toml
[service]
name = "ZisK Distributed Coordinator"
environment = "development"

[server]
host = "0.0.0.0"
port = 50051

[logging]
level = "info"
format = "pretty"
file_path = "coordinator.log"
```

Example production configuration (`production.toml`):
```toml
[service]
name = "ZisK Distributed Coordinator"  
environment = "production"

[server]
host = "0.0.0.0"
port = 50051

[logging]
level = "info"
format = "json"
file_output = true
file_path = "/var/log/coordinator-network/server.log"

[coordinator]
shutdown_timeout_seconds = 30 # Graceful shutdown timeout
max_workers_per_job = 20      # Maximum workers per proof job
max_total_workers = 5000      # Maximum total registered workers  
phase1_timeout_seconds = 600  # 10 minutes for phase 1
phase2_timeout_seconds = 1200 # 20 minutes for phase 2
```

#### Command Line Arguments

```bash
# Show help
cargo run --bin zisk-coordinator -- --help

# Run coordinator with custom port
cargo run --bin zisk-coordinator -- --port 50051

# Run with specific configuration
cargo run --bin zisk-coordinator -- --config production.toml

# Run with webhook URL  
cargo run --bin zisk-coordinator -- --webhook-url http://webhook.example.com/notify
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
[worker]
# worker_id = "my-worker-001"
compute_capacity.compute_units = 10
environment = "development"

[coordinator]
url = "http://127.0.0.1:50051"

[connection]
reconnect_interval_seconds = 5
heartbeat_timeout_seconds = 30

[logging]
level = "info"
format = "pretty"
file_output = false
```

#### Environment Variables

Worker configuration is primarily handled through TOML configuration files and CLI arguments. The current implementation does not use environment variables with the `DISTRIBUTED_` prefix, but relies on:
- Configuration files (TOML)
- Command-line argument overrides
- The `CONFIG_PATH` environment variable for specifying config file location

#### Command Line Arguments

```bash
# Show help and available options
cargo run --bin zisk-worker -- --help

# Override server URL
cargo run --bin zisk-worker -- --coordinator-url "http://production-coordinator:8080"

# Override worker ID  
cargo run --bin zisk-worker -- --worker-id "production-worker-001"

# Override compute capacity
cargo run --bin zisk-worker -- --compute-units 16

# Use custom configuration file
cargo run --bin zisk-worker -- --config production-worker.toml

# Combine multiple overrides
cargo run --bin zisk-worker -- \
  --coordinator-url "http://production-coordinator:8080" \
  --compute-units 32 \
  --worker-id "high-capacity-worker"
```

#### Configuration Priority

Configuration values are resolved in this priority order:

1. **Command line arguments** (highest priority)
2. **Configuration file** (specified by `--config` or default `config.toml`)
3. **Built-in defaults** (lowest priority)

Note: The current implementation does not use environment variables for configuration overrides.

#### Worker Configuration Reference

| Configuration Key | CLI Argument | Environment Variable | Type | Default | Description |
|------------------|--------------|---------------------|------|---------|-------------|
| `coordinator.url` | `--coordinator-url` | - | String | `http://127.0.0.1:50051` | Coordinator server URL |
| `worker.worker_id` | `--worker-id` | - | String | Auto-generated UUID | Unique worker identifier |
| `worker.compute_capacity.compute_units` | `--compute-units` | - | Number | 10 | Available compute units |
| `connection.reconnect_interval_seconds` | - | - | Number | 5 | Reconnection interval in seconds |
| `connection.heartbeat_timeout_seconds` | - | - | Number | 30 | Heartbeat timeout in seconds |
| `worker.environment` | - | - | String | "development" | Worker environment mode |

#### Alternative Configuration File Path

You can specify the configuration file path using an environment variable:

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
cargo run --bin zisk-worker -- --worker-id "dev-worker-2" --compute-units 2
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
     zisk-distributed coordinator
   ```

2. **Worker machines**:
   ```bash
   # Point workers to coordinator IP  
   docker run \
     -v /path/to/worker-config.toml:/app/config.toml \
     zisk-distributed worker --config config.toml
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
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/WorkersList
```

### Scaling Operations

```bash
# Scale up workers in Docker
docker-compose up -d --scale worker=10

# Scale down workers  
docker-compose up -d --scale worker=2

# Add worker on different machine
docker run -d \
  -v /path/to/worker-config.toml:/app/config.toml \
  zisk-distributed worker --config config.toml
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
- Coordinator defaults to port 8080 in config, but Docker examples use 50051
- Use `--port` flag or update configuration file to change ports
- Check for other services using the same ports

### Debug Mode

Enable detailed logging for troubleshooting by modifying configuration files or using CLI arguments:

```bash
# Coordinator with debug logging (via config file)
cargo run --bin zisk-coordinator -- --config debug-coordinator.toml

# Worker with debug logging (via config file)
cargo run --bin zisk-worker -- --config debug-worker.toml
```

Where `debug-coordinator.toml` contains:
```toml
[logging]
level = "debug"
format = "pretty"
```

### Testing Configuration

Use CLI overrides to test specific values without modifying configuration files:

```bash
# Test connection to different coordinator
cargo run --bin zisk-worker -- --coordinator-url http://test-coordinator:50051

# Test with specific capacity and ID
cargo run --bin zisk-worker -- --compute-units 8 --worker-id test-worker

# Test coordinator with different port
cargo run --bin zisk-coordinator -- --port 9090
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


### TODO Webhook

### How the Workers are selected for the `Partial Contributions` and `Prove` phases

The Coordinator selects Workers based on their reported compute capacity and availability. When a proof request is received, the Coordinator evaluates the required compute capacity and selects Workers sequentially from the pool of available Workers until the capacity is met.

### How the Aggregator Worker is selected for the `Aggregation` phase

The first Worker to send its partial proof to the Coordinator is selected as the Aggregator to perform the aggregation of all partial proofs into the final proof.
