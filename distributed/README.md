# ZisK Distributed Proving System

A distributed proof generation system for the ZisK zkVM that orchestrates proof tasks across multiple worker nodes. The system enables horizontal scaling of proof generation workloads, allowing you to distribute computationally intensive proving operations across multiple machines for improved performance and throughput.

## Architecture

The system is composed of two main actors:

- **Coordinator:** Manages incoming proof requests and splits the work, based on required compute capacity, across distributed available workers.  
- **Worker:** Registers to the coordinator, reporting its compute capacity, and waits for tasks to be assigned. A **Worker** can be a single machine or a cluster of machines.

## Proof Generation Process

The process of generating a proof proceeds as follows:  
1. The **Coordinator** starts on a host and listens for incoming proof requests.  
2. **Worker** nodes connect to the Coordinator, registering their compute capacity and availability.  
3. When a proof generation request is received, the Coordinator splits the work across multiple Workers according to the requested compute capacity. A proof generation job is divided into three phases:
   - **Partial Contributions:** Each Worker computes its partial challenges.
   - **Prove:** Workers compute the global challenge and generate their respective partial proofs.  
   - **Aggregation:** A designated Worker aggregates all partial proofs and produces the final proof for the client.
4. The Coordinator collects the final proof and returns it to the client.

### Key Concepts

**Worker Selection:** The Coordinator selects Workers based on their reported compute capacity and availability. When a proof request is received, the Coordinator evaluates the required compute capacity and selects Workers sequentially from the pool of available Workers until the total capacity requirement is met. When a worker is assigned to a job, it is marked as busy and will not receive new tasks until it completes the current job.

**Aggregator Selection:** The first Worker to send its partial proof to the Coordinator is selected as the Aggregator to perform the aggregation of all partial proofs into the final proof. The other Workers are marked as available again after sending their partial proofs.

## Quick Start

### Manual Build and Run

```bash
# Build binaries (from project root)
cargo build --release --bin zisk-coordinator --bin zisk-worker

# Run coordinator
cargo run --release --bin zisk-coordinator

# Run a worker node (in another terminal)
cargo run --release --bin zisk-worker -- --elf <elf-file-path> --inputs-folder <inputs-folder>

# Generate a proof (in another terminal)
cargo run --release --bin zisk-coordinator prove --input <input-filename> --compute-capacity 10
```

### Docker Deployment

The easiest way to run the distributed system:

```bash
# 0. Build the Docker image (CPU-only, default)
docker build -t zisk-distributed:latest -f distributed/Dockerfile .

# 0b. Build with GPU support (if needed)
docker build --build-arg GPU=true -t zisk-distributed:gpu -f distributed/Dockerfile .

# Create a user-defined network so container names resolve via DNS
docker network create zisk-net || true

# 1. Start coordinator container (detached)
LOGS_DIR="<logs-folder>"
docker run -d --rm --name zisk-coordinator \
  --network zisk-net \
  -v "$LOGS_DIR:/var/log/distributed" \
  -e RUST_LOG=info \
  zisk-distributed:latest \
  zisk-coordinator --config /app/config/coordinator/dev.toml

# 2. View coordinator logs
docker logs -f zisk-coordinator

# 3. Start worker container(s) in a different terminal(s) - they connect to coordinator by container name
# Replace paths with your actual directories
LOGS_DIR="<logs-folder>"
PROVING_KEY_DIR="<provingKey-folder>"
ELF_DIR="<elf-folder>"
INPUTS_DIR="<inputs-folder>"
docker run -d --rm --name zisk-worker-1 \
  --network zisk-net --shm-size=20g \
  -v "$LOGS_DIR:/var/log/distributed" \
  -v "$HOME/.zisk/cache:/app/.zisk/cache:ro" \
  -v "$PROVING_KEY_DIR:/app/proving-keys:ro" \
  -v "$ELF_DIR:/app/elf:ro" \
  -v "$INPUTS_DIR:/app/inputs:ro" \
  -e RUST_LOG=info \
  zisk-distributed:latest zisk-worker --coordinator-url http://zisk-coordinator:50051 \
    --elf /app/elf/zec.elf --proving-key /app/proving-keys --inputs-folder /app/inputs

# 4. View coordinator logs
docker logs -f zisk-worker-1

# Generate a proof (use filename only, not full path)
docker exec -it zisk-coordinator \
  zisk-coordinator prove --input <input-filename> --compute-capacity 10

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

## Coordinator

The coordinator is responsible for managing the distributed proof generation process. It receives proof requests from clients and assigns work to available workers.

To start a coordinator instance with default settings:

```bash
cargo run --release --bin zisk-coordinator
```

### Coordinator Configuration

The coordinator can be configured using either a **TOML configuration file** or **command-line arguments**.
If no configuration file is explicitly provided, the system falls back to the `ZISK_COORDINATOR_CONFIG_PATH` environment variable to locate one. If neither the CLI argument nor environment variable is set, built-in defaults are used.

**Example:**

```bash
# You can specify the configuration file path using a command line argument:
cargo run --release --bin zisk-coordinator -- --config /path/to/my-config.toml

# You can specify the configuration file path using an environment variable:
export ZISK_COORDINATOR_CONFIG_PATH="/path/to/my-config.toml"
cargo run --release --bin zisk-coordinator
```

The table below lists the available configuration options for the Coordinator:

| TOML Key              | CLI Argument     | Environment Variable| Type | Default | Description |
|-----------------------|--------------|---------------------|------|---------|-------------|
| `service.name` | - | - | String | ZisK Distributed Coordinator | Service name |
| `service.environment` | - | - | String | development | Service environment (development, staging, production) |
| `server.host` | - | - | String | 0.0.0.0 | Server host |
| `server.port` | `--port` | - | Number | 50051 | Server port |
| `server.shutdown_timeout_seconds` | - | - | Number | 30 | Graceful shutdown timeout in seconds |
| `logging.level` | - | RUST_LOG | String | debug | Logging level (error, warn, info, debug, trace) |
| `logging.format` | - | - | String | pretty | Logging format (pretty, json, compact) |
| `logging.file_path` | - | - | String | - | *Optional*. Log file path (enables file logging) |
| `coordinator.max_workers_per_job` | - | - | Number | 10 | Maximum workers per proof job |
| `coordinator.max_total_workers` | - | - | Number | 1000 | Maximum total registered workers |
| `coordinator.phase1_timeout_seconds` | - | - | Number | 300 | Phase 1 timeout in seconds |
| `coordinator.phase2_timeout_seconds` | - | - | Number | 600 | Phase 2 timeout in seconds |
| `coordinator.webhook_url` | `--webhook-url` | - | String | - | *Optional*. Webhook URL to notify on job completion |


#### Configuration Files examples

Example development configuration file:

```toml
[service]
name = "ZisK Distributed Coordinator"
environment = "development"

[logging]
level = "debug"
format = "pretty"
```

Example production configuration file:

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
file_path = "/var/log/distributed/coordinator.log"

[coordinator]
max_workers_per_job = 20      # Maximum workers per proof job
max_total_workers = 5000      # Maximum total registered workers  
phase1_timeout_seconds = 600  # 10 minutes for phase 1
phase2_timeout_seconds = 1200 # 20 minutes for phase 2
webhook_url = "http://webhook.example.com/notify?job_id={$job_id}"
```

### Webhook URL

The Coordinator can notify an external service when a job finishes by sending a request to a configured webhook URL.
The placeholder {$job_id} can be included in the URL and will be replaced with the finished job’s ID.
If no placeholder is provided, the Coordinator automatically appends /{job_id} to the end of the URL.

All webhook notifications are sent as JSON POST requests with the following structure:

```json
{
  "job_id": "job_12345",
  "success": true,
  "duration_ms": 45000,
  "proof": <array of u64...>,
  "timestamp": "2025-10-03T14:30:00Z",
  "error": null
}
```

##### Fields Description

| Field | Type | Description |
|-------|------|-------------|
| `job_id` | `string` | Unique identifier for the proof generation job |
| `success` | `boolean` | `true` if proof generation completed successfully, `false` if it failed |
| `duration_ms` | `number` | Total execution time in milliseconds from job start to completion |
| `proof` | `array<u64>` \| `null` | Final proof data as array of integers (only present on success) |
| `timestamp` | `string` | ISO 8601 timestamp when the notification was sent |
| `error` | `object` \| `null` | Error details (only present on failure) |

##### Error Object Structure

When `success` is `false`, the `error` field contains:

```json
{
  "code": "WORKER_FAILURE",
  "message": "Worker node-003 failed during proof generation: Out of memory"
}
```

**Successful Proof Generation Example::**

```json
{
  "job_id": "job_abc123",
  "success": true,
  "duration_ms": 32500,
  "proof": [1234567890, 9876543210, 1357924680, ...],
  "timestamp": "2025-10-03T14:30:25Z",
  "error": null
}
```

**Failed Job Example:**

```json
{
  "job_id": "job_def456",
  "success": false,
  "duration_ms": 15000,
  "proof": null,
  "timestamp": "2025-10-03T14:31:10Z",
  "error": {
    "code": "WORKER_ERROR",
    "message": "Memory exhaustion during proof generation"
  }
}
```

#### Webhook Implementation Guidelines

*HTTP Requirements:*

- **Method**: POST
- **Content-Type**: `application/json`
- **Timeout**: 10 seconds (configurable)
- **Retry**: Currently no automatic retries (implement idempotency)

*Recommended Response:*

Your webhook endpoint should respond with:

- **Success**: HTTP 200-299 status code
- **Body**: Any valid response (ignored by coordinator)

```http
HTTP/1.1 200 OK
Content-Type: application/json

{"received": true, "job_id": "job_abc123"}
```

If your webhook endpoint is unavailable or returns an error:

- The coordinator logs the failure but continues operation
- No automatic retries are performed
- Consider implementing your own retry mechanism or message queue

### Command Line Arguments

```bash
# Show help
cargo run --release --bin zisk-coordinator -- --help

# Run coordinator with custom port
cargo run --release --bin zisk-coordinator -- --port 50051

# Run with specific configuration
cargo run --release --bin zisk-coordinator -- --config production.toml

# Run with webhook URL  
cargo run --release --bin zisk-coordinator -- --webhook-url http://webhook.example.com/notify --port 50051
```

## Worker

The worker is responsible for executing proof generation tasks assigned by the coordinator. It registers with the coordinator, reports its compute capacity, and waits for tasks to be assigned.

To start a worker instance with default settings:

```bash
cargo run --release --bin zisk-worker -- --elf <elf-file-path> --inputs-folder <inputs-folder>
```

### Worker Configuration

The worker can be configured using either a **TOML configuration file** or **command-line arguments**.
If no configuration file is explicitly provided, the system falls back to the `ZISK_WORKER_CONFIG_PATH` environment variable to locate one. If neither the CLI argument nor environment variable is set, built-in defaults are used.

**Example:**

```bash
# You can specify the configuration file path using a command line argument:
cargo run --release --bin zisk-worker -- --config /path/to/my-config.toml

# You can specify the configuration file path using an environment variable:
export ZISK_WORKER_CONFIG_PATH="/path/to/my-config.toml"
cargo run --release --bin zisk-worker
```

### Input Files Handling

Workers need to know where to find input files for proof generation. The `--inputs-folder` parameter specifies the base directory where input files are stored:

- **Default**: Current working directory (`.`) if not specified
- **Usage**: When the coordinator sends a prove command with an input filename, the worker combines `--inputs-folder` + `filename` to locate the file
- **Benefits**: Allows input files to be organized in a dedicated directory, separate from the worker executable

**Example:**
```bash
# Worker with inputs in specific folder
cargo run --release --bin zisk-worker -- --elf program.elf --inputs-folder /data/inputs/

# Coordinator requests proof for "input.bin" -> Worker looks for "/data/inputs/input.bin"
cargo run --release --bin zisk-coordinator -- prove --input input.bin --compute-capacity 10
```

The table below lists the available configuration options for the Worker:

| TOML Key              | CLI Argument     | Environment Variable| Type | Default | Description |
|-----------------------|--------------|---------------------|------|---------|-------------|
| `worker.worker_id` | `--worker-id` | - | String | Auto-generated UUID | Unique worker identifier |
| `worker.compute_capacity.compute_units` | `--compute-capacity` | - | Number | 10 | Worker compute capacity (in compute units) |
| `worker.environment` | - | - | String | development | Service environment (development, staging, production) |
| `worker.inputs_folder` | `--inputs-folder` | - | String | . | Path to folder containing input files |
| `coordinator.url` | `--coordinator-url` | - | String | http://127.0.0.1:50051 | Coordinator server URL |
| `connection.reconnect_interval_seconds` | - | - | Number | 5 | Reconnection interval in seconds |
| `connection.heartbeat_timeout_seconds` | - | - | Number | 30 | Heartbeat timeout in seconds |
| `logging.level` | - | RUST_LOG | String | debug | Logging level (error, warn, info, debug, trace) |
| `logging.format` | - | - | String | pretty | Logging format (pretty, json, compact) |
| `logging.file_path` | - | - | String | - | *Optional*. Log file path (enables file logging) |
| - | `--witness-lib` | - | String | ~/.zisk/bin/libzisk_witness.so | Path to witness computation dynamic library |
| - | `--proving-key` | - | String | ~/.zisk/provingKey | Path to setup folder |
| - | `--elf` | - | String | - | Path to ELF file |
| - | `--asm` | - | String | ~/.zisk/cache | Path to ASM file (mutually exclusive with `--emulator`) |
| - | `--emulator` | - | Boolean | false | Use prebuilt emulator (mutually exclusive with `--asm`) |
| - | `--asm-port` | - | Number | 23115 | Base port for Assembly microservices |
| - | `--shared-tables` | - | Boolean | false | Whether to share tables when worker is running in a cluster |
| - | `-v`, `-vv`, `-vvv`, ... | - | Number | 0 | Verbosity level (0=error, 1=warn, 2=info, 3=debug, 4=trace) |
| - | `-d`, `--debug` | - | String | - | Enable debug mode with optional component filter |
| - | `--verify-constraints` | - | Boolean | false | Whether to verify constraints |
| - | `--unlock-mapped-memory` | - | Boolean | false | Unlock memory map for the ROM file (mutually exclusive with `--emulator`) |
| - | `-f`, `--final-snark` | - | Boolean | false | Whether to generate the final SNARK |
| - | `-r`, `--preallocate` | - | Boolean | false | GPU preallocation flag |
| - | `-t`, `--max-streams` | | - | Number | - | Maximum number of GPU streams |
| - | `-n`, `--number-threads-witness` | - | Number | - | Number of threads for witness computation |
| - | `-x`, `--max-witness-stored` | - | Number | - | Maximum number of witnesses to store in memory |

#### Configuration Files examples

Example development configuration file:

```toml
[worker]
compute_capacity.compute_units = 10
environment = "development"

[logging]
level = "debug"
format = "pretty"
```

Example production configuration file:

```toml
[worker]
worker_id = "my-worker-001"
compute_capacity.compute_units = 10
environment = "production"
inputs_folder = "/app/inputs"

[coordinator]
url = "http://127.0.0.1:50051"

[connection]
reconnect_interval_seconds = 5
heartbeat_timeout_seconds = 30

[logging]
level = "info"
format = "pretty"
file_path = "/var/log/distributed/worker-001.log"
```

## Launching a proof

To launch a proof generation request, use the `prove` command of the `zisk-coordinator` binary, specifying the input filename and desired compute capacity.

```bash
cargo run --release --bin zisk-coordinator -- prove --input <input_filename> --compute-capacity 10
```

The `--compute-capacity` flag indicates the total compute units required to generate a proof. The coordinator will assign one or more workers to meet this capacity, distributing the workload if multiple workers are needed. Requests exceeding the combined capacity of available workers will not be processed and an error will be returned.

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

## Troubleshooting

### Common Issues

**Worker can't connect to coordinator:**
- Verify coordinator is running and accessible on the specified port
- Check firewall settings if coordinator and worker are on different machines
- Ensure correct URL format: `http://host:port` (not `https://` for default setup)

**Configuration not loading:**
- Verify TOML syntax with a TOML validator
- Check file permissions on configuration files
- Use CLI overrides to test specific values

**Worker not receiving tasks:**
- Check worker registration in coordinator logs
- Verify compute capacity is appropriate for available tasks
- Ensure worker ID is unique if running multiple workers
- Confirm coordinator has active jobs to distribute

**Input file not found errors:**
- Verify the input file exists in the worker's `--inputs-folder` directory
- Check file permissions - worker needs read access to input files
- Ensure you're using the filename only (not full path) when launching proofs
- Confirm `--inputs-folder` path is correct and accessible

**Port conflicts:**
- Use `--port` flag or update configuration file to change ports
- Check for other services using the same ports

### Debug Mode

Enable detailed logging for troubleshooting by modifying configuration files or using CLI arguments:

```bash
# Coordinator with debug logging (via config file)
cargo run --release --bin zisk-coordinator -- --config debug-coordinator.toml

# Worker with debug logging (via config file)
cargo run --release --bin zisk-worker -- --config debug-worker.toml
```

Where `debug-coordinator.toml` or `debug-worker.toml` contains:
```toml
[logging]
level = "debug"
format = "pretty"
```

### Log Files

When file logging is enabled, logs are written into specified paths in the configuration files. Ensure the application has write permissions to these paths.

```toml
[logging]
file_path = "/var/log/distributed/coordinator.log"
```
