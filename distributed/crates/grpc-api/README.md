# Distributed Proving System gRPC API

This crate provides the gRPC API layer for ZisK's distributed proving system, enabling coordination between proof coordinators and distributed provers.

## Core gRPC Methods

### 1. ProverStream (Bidirectional Streaming)

The main communication channel between coordinator and provers. This bidirectional stream handles:

**Prover → Coordinator Messages:**
- `ProverRegisterRequest` - Initial prover registration with compute capacity
- `ProverReconnectRequest` - Reconnection with last known state  
- `ExecuteTaskResponse` - Task completion results (challenges, proofs, final proofs)
- `HeartbeatAck` - Heartbeat acknowledgments
- `ProverError` - Error reporting

**Coordinator → Prover Messages:**
- `ProverRegisterResponse` - Registration confirmation with assigned prover ID
- `ExecuteTaskRequest` - Task assignment (contribution, prove, aggregate)
- `Heartbeat` - Keep-alive messages
- `JobCancelled` - Job cancellation notifications
- `Shutdown` - Graceful shutdown requests

### 2. Administrative Endpoints

**StatusInfo()** - Get service information including uptime, version, metrics
**HealthCheck()** - Basic health check for monitoring
**JobsList(active_only)** - List current and historical proof jobs
**ProversList(available_only)** - List connected provers and their states
**JobStatus(job_id)** - Get detailed status of a specific job
**SystemStatus()** - Overall system metrics (active jobs, prover counts, capacity)
**LaunchProof(block_id, compute_capacity, input_path)** - Start a new proof job

## Task Types

The system supports three types of proof tasks:

1. **PARTIAL_CONTRIBUTION** - Distributed witness generation
2. **PROVE** - Generate cryptographic proofs from challenges  
3. **AGGREGATE** - Combine partial proofs into final proof

## Testing with grpcurl

Install grpcurl for testing:
```bash
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest
```

### Basic Health and Status Checks

**Note**: Default coordinator port is `8080`, but Docker deployment uses `50051`.

```bash
# Health check (adjust port as needed)
grpcurl -plaintext 127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/HealthCheck

# Get service information  
grpcurl -plaintext 127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/StatusInfo

# Get system status
grpcurl -plaintext 127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/SystemStatus

# For manual builds using default port
grpcurl -plaintext 127.0.0.1:8080 zisk.distributed.api.v1.ZiskDistributedApi/HealthCheck
```

### Job and Prover Management

```bash
# List active jobs
grpcurl -plaintext -d '{"active_only": true}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/JobsList

# List available provers
grpcurl -plaintext -d '{"available_only": true}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/ProversList

# Get specific job status
grpcurl -plaintext -d '{"job_id": "job_123"}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/JobStatus

# Launch a new proof job
grpcurl -plaintext -d '{"block_id": "block_456", "compute_capacity": 4, "input_path": "/path/to/input"}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/LaunchProof
```

### Interactive Streaming (Advanced)

For testing the bidirectional `ProverStream`, you'll need a more sophisticated client. The stream handles prover registration and task execution:

```bash
# Example prover registration message (requires streaming client)
# {
#   "register": {
#     "prover_id": "prover_001", 
#     "compute_capacity": {"compute_units": 8}
#   }
# }
```

## Setup and Deployment

### Prerequisites

- Rust toolchain (see `rust-toolchain.toml`)
- Protocol Buffers compiler (`protoc`)
- Docker & Docker Compose (for containerized deployment)

### Docker Deployment (Recommended)

The easiest way to run the distributed system is using Docker Compose:

```bash
# From the distributed directory
cd /path/to/zisk/distributed

# Build and start services
docker-compose up -d

# View logs  
docker-compose logs -f coordinator
docker-compose logs -f prover

# Scale prover nodes
docker-compose up -d --scale prover=3

# Stop services
docker-compose down
```

**Docker Configuration:**
- **Coordinator**: Runs on port `50051` (configured via `DISTRIBUTED_SERVER_PORT=50051`)
- **Prover**: Connects to `http://coordinator:50051` 
- **Environment**: Uses `DISTRIBUTED_` prefix for environment variables

### Manual Building and Running

```bash
# Build binaries
cargo build --release --bin coordinator --bin prover

# Run coordinator (defaults to port 8080, override with --port)
cargo run --bin coordinator -- server --port 50051

# Run prover with custom server URL  
cargo run --bin prover -- --url http://127.0.0.1:50051
```

### Configuration

**Environment Variables** (used by Docker):
- `DISTRIBUTED_SERVER_HOST` - Coordinator bind host (default: `0.0.0.0`)
- `DISTRIBUTED_SERVER_PORT` - Coordinator port (default: `8080`, Docker uses `50051`)  
- `DISTRIBUTED_SERVER_URL` - Prover connection URL (e.g., `http://coordinator:50051`)
- `DISTRIBUTED_PROVER_ID` - Unique prover identifier
- `DISTRIBUTED_LOGGING_LEVEL` - Log level (`info`, `debug`, etc.)
- `DISTRIBUTED_LOGGING_FORMAT` - Log format (`json`, `pretty`, `compact`)

**Note**: The coordinator code currently expects `CONSENSUS_` prefix for environment variables, but Docker deployment uses `DISTRIBUTED_` prefix. This discrepancy exists in the current implementation.

### Integration

This crate provides the generated gRPC stubs and message types. Import it in your coordinator or prover implementations:

```rust
use distributed_grpc_api::zisk_distributed_api::{
    ZiskDistributedApiServer, ZiskDistributedApiClient,
    ProverMessage, CoordinatorMessage,
    // ... other message types
};
```

### Known Configuration Issues

⚠️ **Environment Variable Prefix Mismatch**: The current implementation has a discrepancy between Docker deployment and code configuration:

- **Docker Compose** uses `DISTRIBUTED_` prefix (e.g., `DISTRIBUTED_SERVER_PORT`)  
- **Coordinator code** expects `CONSENSUS_` prefix for environment variables
- **Default ports** differ: coordinator defaults to `8080`, Docker uses `50051`

This affects environment variable configuration and may require code updates for full Docker compatibility.

## Error Handling

All admin endpoints use a standardized error response format:

```proto
message ErrorResponse {
  string code = 1;      // Error code (e.g., "JOB_NOT_FOUND")
  string message = 2;   // Human-readable error message  
}
```

Common error codes:
- `JOB_NOT_FOUND` - Requested job doesn't exist
- `PROVER_UNAVAILABLE` - No provers available for task
- `SYSTEM_UNAVAILABLE` - System temporarily unavailable

## Protocol Buffer Schema

The complete gRPC service definition is available in `proto/zisk_distributed_api.proto` with detailed message schemas for all request/response types.
