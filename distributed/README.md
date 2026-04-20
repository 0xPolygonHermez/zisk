# ZisK Distributed Proving System

A distributed proof generation system for the ZisK zkVM that orchestrates proof tasks across multiple worker nodes. The system enables horizontal scaling of proof generation workloads, allowing you to distribute computationally intensive proving operations across multiple machines for improved performance and throughput.

## Architecture

The system has two components:

- **Coordinator** `zisk-coordinator`: The single entry point for clients. Exposes a public gRPC API, hosts the coordination engine, and listens for worker connections on a separate internal port.
- **Worker** `zisk-worker`: Registers to the coordinator, reports its compute capacity, and executes assigned proof tasks.

```
             Client
               │
               │  gRPC (port 7000)
               ▼
┌──────────────────────────────┐
│       zisk-coordinator       │
│                              │
│  ┌────────────────────────┐  │
│  │   Coordination Engine  │  │◄── zisk-worker (port 50051, N workers)
│  └────────────────────────┘  │
│                              │
│   Prometheus metrics :9090   │
└──────────────────────────────┘
```

## Proof Generation Process

1. The **Coordinator** starts opening both the public API port and the internal cluster port.
2. **Worker** nodes connect to the cluster's internal port, registering their compute capacity.
3. When a proof request arrives via the public API, the coordinator selects workers and assigns each a partition of the total compute capacity. Each worker uses its assigned partition to independently determine which part of the computation to perform — the coordinator does not split the work itself, only the assignment. A proof job proceeds through three phases:
   - **Contributions:** Each worker computes its partial challenges for its assigned partition.
   - **Prove:** Each worker uses the global challenge to generate its partial proof.
   - **Aggregation:** A designated worker aggregates all partial proofs into the final proof.
4. The coordinator returns the final proof to the client.

## Key Concepts

**Worker Selection:** Workers are selected based on their reported compute capacity and availability. When a proof request arrives, the coordinator evaluates the required capacity and picks workers sequentially from the idle pool until the total capacity requirement is met. Selected workers are marked busy for the duration of the job and will not receive new tasks until they finish.

**Aggregator Selection:** The first worker to deliver its partial proof becomes the aggregator for that job. It combines all partial proofs into the final proof while the remaining workers are released back to the idle pool immediately after submitting their partial proofs.

## Quick Start

### Manual Build and Run

```bash
# Build binaries (from project root)
cargo build --release --bin zisk-coordinator --bin zisk-worker

# Start the coordinator (listens for workers on :50051)
cargo run --release --bin zisk-coordinator

# Start a worker (in another terminal) — connects to the coordinator port
cargo run --release --bin zisk-worker -- --config distributed/crates/worker/config/dev.toml
```

### Docker Deployment

The easiest way to run the distributed system. All commands run from the **workspace root**.

```bash
# Build images
docker compose -f distributed/docker-compose.yml build

# Start the coordinator only
docker compose -f distributed/docker-compose.yml up coordinator

# Start the full stack: coordinator + 4 workers + Prometheus
docker compose -f distributed/docker-compose.yml up --scale worker=4

# Build workers with GPU support
docker compose -f distributed/docker-compose.yml build --build-arg GPU=true worker
docker compose -f distributed/docker-compose.yml up --scale worker=4
```

**Port mapping:**

| Port | Service |
|------|---------|
| `7000` | Public gRPC API (clients connect here) |
| `9090` | Prometheus metrics (coordinator) |
| `9091` | Prometheus UI (mapped from the prometheus container) |
| `50051` | Internal worker port (workers connect here, not exposed externally by default) |

**Volumes:**

Uncomment the cache volume in `docker-compose.yml` to mount proving keys and ELF cache from the host:

```yaml
volumes:
  - ${ZISK_CACHE_DIR:-~/.zisk/cache}:/app/.zisk/cache:ro
```

## Coordinator Configuration

The coordinator is configured via a TOML file. The search order (later entries override earlier):

1. `/etc/zisk/coordinator.toml` — system-wide
2. `$XDG_CONFIG_HOME/zisk/coordinator.toml` — user-level
3. `./coordinator.toml` — current directory
4. `ZISK_COORDINATOR_*` environment variables
5. CLI flags

| TOML Key | CLI / Env | Default | Description |
|---|---|---|---|
| `service.name` | — | `ZisK Coordinator` | Service name |
| `service.environment` | — | `development` | `development` \| `staging` \| `production` |
| `server.host` | — | `0.0.0.0` | gRPC listen host |
| `server.port` | `--port` / `ZISK_COORDINATOR_PORT` | `7000` | gRPC listen port |
| `server.shutdown_timeout_seconds` | — | `30` | Graceful shutdown timeout |
| `metrics.enabled` | — | `true` | Enable Prometheus metrics endpoint |
| `metrics.host` | — | `0.0.0.0` | Metrics listen host |
| `metrics.port` | — | `9090` | Metrics listen port |
| `logging.level` | `--log-level` / `RUST_LOG` | `info` | `trace` \| `debug` \| `info` \| `warn` \| `error` |
| `logging.format` | — | `pretty` | `pretty` \| `json` \| `compact` |
| `backend.mode` | `--backend` / `ZISK_COORDINATOR_BACKEND` | `coordinator` | `coordinator` \| `mock` |
| `coordinator.worker_port` | — | `50051` | Port for worker connections |
| `coordinator.config_file` | — | — | Optional path to a coordinator TOML config |

#### Example: development config

```toml
[service]
environment = "development"

[logging]
level  = "debug"
format = "pretty"

[backend]
mode = "coordinator"

[coordinator]
worker_port = 50051
```

#### Example: production config

```toml
[service]
environment = "production"

[server]
port = 7000

[metrics]
enabled = true
port    = 9090

[logging]
level  = "info"
format = "json"

[backend]
mode = "coordinator"

[coordinator]
worker_port = 50051
# config_file = "/etc/zisk/coordinator.toml"  # tune coordinator internals
```

### Coordinator tuning (optional)

Advanced coordinator parameters can be provided via a separate TOML file referenced by `coordinator.config_file`. Example files are in `distributed/crates/coordinator/config/`.

## Worker

The worker executes proof generation tasks assigned by the coordinator. It connects to the coordinator's worker port, registers its compute capacity, and waits for task assignments. Workers reconnect automatically on disconnection.

```bash
# Start a worker (connects to coordinator at 127.0.0.1:50051 by default)
cargo run --release --bin zisk-worker -- --config worker.toml

# Equivalent with individual flags
cargo run --release --bin zisk-worker -- \
  --coordinator-url http://<coordinator-host>:50051 \
  --compute-capacity 10 \
  --proving-key ~/.zisk/provingKey

# GPU-accelerated worker
cargo run --release --bin zisk-worker -- --config worker.toml --hints --shared-tables
```

You can run multiple workers on the same machine by pointing each to a different config file. Each worker must have a unique `worker_id` (auto-generated UUID if unset).

## Worker Configuration

Workers are configured via a TOML file. The search order follows the same pattern as the coordinator, using the `ZISK_WORKER_CONFIG` environment variable.

| TOML Key | CLI / Env | Default | Description |
|---|---|---|---|
| `worker.worker_id` | `--worker-id` | Auto UUID | Unique worker identifier |
| `worker.compute_capacity.compute_units` | `--compute-capacity` | `10` | Compute capacity in units |
| `worker.environment` | — | `development` | `development` \| `production` |
| `coordinator.url` | `--coordinator-url` | `http://127.0.0.1:50051` | gRPC URL of the coordinator's worker-facing port |
| `connection.reconnect_interval_seconds` | — | `5` | Reconnection interval |
| `connection.heartbeat_timeout_seconds` | — | `30` | Heartbeat timeout |
| `logging.level` | `RUST_LOG` | `info` | Log level |
| `logging.format` | — | `pretty` | `pretty` \| `json` \| `compact` |
| `logging.file_path` | — | — | Optional log file path |

Additional CLI-only flags:

| CLI Argument | Default | Description |
|---|---|---|
| `--proving-key` | `~/.zisk/provingKey` | Path to the setup (proving key) folder |
| `--elf` | — | Path to ELF file |
| `--asm` | `~/.zisk/cache` | Path to ASM file |
| `--hints` | `false` | Enable precompile hints processing |
| `--shared-tables` | `false` | Share tables when running in a cluster |
| `--verify-constraints` | `false` | Verify constraints after witness generation |
| `-n`, `--number-threads-witness` | — | Threads for witness computation |
| `-t`, `--max-streams` | — | Maximum GPU streams |

#### Example: production config

```toml
[worker]
compute_capacity.compute_units = 10
environment = "production"

[coordinator]
url = "http://<coordinator-host>:50051"

[connection]
reconnect_interval_seconds = 5
heartbeat_timeout_seconds  = 30

[logging]
level  = "info"
format = "json"
file_path = "/var/log/zisk/worker.log"
```

## Coordinator API

The coordinator exposes a gRPC service `ZiskCoordinatorApi` on port 7000. Follow the proto definitions in `distributed/crates/coordinator-api/proto/zisk_coordinator_api.proto` for the full API specification. The main RPC methods are summarized below.

### Operations

#### `RegisterGuestProgram`

Uploads a ZisK ELF binary to the coordinator. Returns a stable `hash_id` (blake3 of the ELF bytes). Idempotent — the same ELF always returns the same `hash_id`. Registering a program stores the ELF binary in the coordinator's cache and makes it available for subsequent setup jobs. The `hash_id` is used to reference the program in later requests.

#### `JobRequest` — Setup

Distributes the ELF binary to the workers to generate all necessary proving artifacts and load them in preparation for execution. This is necessary for any program to run on the cluster. Must be called once before any job is launched versus that program. The `hash_id` references the ELF registered via `RegisterGuestProgram`. Idempotent — calling setup multiple times with the same `hash_id` is a no-op after the first successful setup.

#### `JobRequest` — Execute

Runs the program with the given inputs and returns execution statistics and public outputs, **without generating a proof**. Useful for dry-runs, cost estimation, or output-only use cases.

#### `JobRequest` — Prove

Runs the program and generates a proof. The `proof_dest` field controls the output format and generation method. At this moment the coordinator supports three proof types: STARK, STARK Minimal, and PLONK.

**Input kinds:**

- `inline` — send the full input inline in the request. For large inputs, set `is_last=false` on the first chunk and stream the remainder with `PushJobInput`.
- `stream_uri` — point to a URI (`file://`, `unix://`, `quic://`) the internal coordinator can fetch directly.

#### `JobRequest` — Wrap

Converts a proof into another proof type. Takes the `Proof` object returned by a Prove job.

### Job monitoring

Both methods work for any job kind. Use whichever fits your client model:

| Method | Style | Use when |
|---|---|---|
| `WaitJobResult` | Long-poll (request/response) | Simple clients; loop until terminal |
| `WatchJob` | Server-streaming | Event-driven clients; real-time progress |

**`WaitJobResult`** blocks server-side for up to `timeout_seconds` (1–3600 s, default 5 s), then returns the current status. If the status is still `Running`, call again — no application-level sleep needed.

**`WatchJob`** opens a stream that emits one `JobEvent` per state transition and closes automatically on the terminal event (`Completed`, `Failed`, or `Cancelled`). Safe to call after the job has already finished.

Job states: `Queued` → `Running(phase)` → `WaitingForInput`? → `Completed` | `Failed` | `Cancelled`

Phases within `Running`: `Contributions` → `Prove` → `Aggregate`

### `PushJobInput`

Stream additional input chunks to a job.

### `CancelJob`

Cancels a running or queued job.

## Health Checking

The coordinator implements the [gRPC Health Checking Protocol](https://github.com/grpc/grpc/blob/master/doc/health-checking.md):

## Running as a System Service

For production deployments without Docker, use the install scripts to register the binaries as systemd services. The scripts create a dedicated system user, install the config, write the unit file, and start the service.

### Coordinator

```bash
# Build from source and install (run from workspace root)
sudo distributed/crates/coordinator-server/scripts/install.sh

# Use a pre-built binary
sudo distributed/crates/coordinator-server/scripts/install.sh --binary target/release/zisk-coordinator

# Use an existing config file
sudo distributed/crates/coordinator-server/scripts/install.sh --config /path/to/coordinator.toml

# Remove the service
sudo distributed/crates/coordinator-server/scripts/install.sh --uninstall
```

```bash
sudo journalctl -u zisk-coordinator -f
```

### Worker

```bash
# Build from source and install
sudo distributed/crates/worker/scripts/install.sh

# Specify the proving key directory
sudo distributed/crates/worker/scripts/install.sh --proving-key /mnt/data/provingKey

# Use a pre-built binary and existing config
sudo distributed/crates/worker/scripts/install.sh \
  --binary target/release/zisk-worker \
  --config /path/to/worker.toml

# Remove the service
sudo distributed/crates/worker/scripts/install.sh --uninstall
```

```bash
sudo journalctl -u zisk-worker -f
```

**Multiple workers per host** — install each instance with a different config file that sets a unique `worker_id` (or leaves it unset to auto-generate a UUID). Use systemd's template units to manage them together:

```bash
# Copy the generated unit to a template:
sudo cp /etc/systemd/system/zisk-worker.service \
        /etc/systemd/system/zisk-worker@.service
# Edit ExecStart in the template to use %i:
#   ExecStart=/usr/local/bin/zisk-worker --config /etc/zisk/worker-%i.toml

sudo systemctl enable --now zisk-worker@1
sudo systemctl enable --now zisk-worker@2
```

## Troubleshooting

**Worker can't connect to coordinator:**
- Verify the coordinator is running and the worker port is accessible (`50051` by default)
- In Docker, confirm both services are on the same network (`zisk`)
- Ensure `coordinator.url` in `worker.toml` matches the coordinator's hostname and worker port

**Coordinator fails to start:**
- Check for port conflicts on `7000`, `9090`, or `50051`
- Validate `coordinator.toml` with a TOML linter
- Use `--log-level debug` for detailed startup logging

**Worker not receiving tasks:**
- Check worker registration in coordinator logs (`RUST_LOG=debug`)
- Verify the worker's `compute_capacity` is sufficient for queued jobs
- Ensure no two workers share the same `worker_id`

**Debug logging:**

```bash
# Coordinator
RUST_LOG=debug cargo run --release --bin zisk-coordinator

# Worker
RUST_LOG=debug cargo run --release --bin zisk-worker -- --config worker.toml
```

Or via config file:

```toml
[logging]
level  = "debug"
format = "pretty"
```
