# gRPC API Layer

This crate provides the gRPC protocol definitions and generated stubs for communication between the ZisK distributed proving system coordinator and workers.

> **Note**: For general setup, deployment, and configuration instructions, see the main [distributed system README](../../README.md).

## Protocol Overview

The API uses bidirectional gRPC streaming for real-time communication between coordinator and workers, plus unary RPCs for administrative operations.

## Core gRPC Service

### ProverStream (Bidirectional Streaming)

The primary communication channel between coordinator and workers:

**Worker → Coordinator Messages:**
- `ProverRegisterRequest` - Initial worker registration with compute capacity
- `ProverReconnectRequest` - Reconnection with last known state  
- `ExecuteTaskResponse` - Task completion results (challenges, proofs, final proofs)
- `HeartbeatAck` - Heartbeat acknowledgments
- `ProverError` - Error reporting

**Coordinator → Worker Messages:**
- `ProverRegisterResponse` - Registration confirmation with assigned worker ID
- `ExecuteTaskRequest` - Task assignment (contribution, prove, aggregate)
- `Heartbeat` - Keep-alive messages
- `JobCancelled` - Job cancellation notifications
- `Shutdown` - Graceful shutdown requests

### Task Types

The system supports three types of proof tasks:

1. **PARTIAL_CONTRIBUTION** - Distributed witness generation
2. **PROVE** - Generate cryptographic proofs from challenges  
3. **AGGREGATE** - Combine partial proofs into final proof

## Administrative API

### Unary RPC Methods

These methods provide monitoring and control capabilities:

- **`HealthCheck()`** - Basic service health status
- **`StatusInfo()`** - Service information including uptime, version, metrics
- **`SystemStatus()`** - Overall system metrics (active jobs, worker counts, capacity)
- **`JobsList(JobsListRequest)`** - List current and historical proof jobs
- **`ProversList(ProversListRequest)`** - List connected workers and their states
- **`JobStatus(JobStatusRequest)`** - Get detailed status of a specific job
- **`LaunchProof(LaunchProofRequest)`** - Start a new proof job

### Error Handling

All administrative endpoints use standardized error responses:

```proto
message ErrorResponse {
  string code = 1;      // Error code (e.g., "JOB_NOT_FOUND")
  string message = 2;   // Human-readable error message  
}
```

Common error codes:
- `JOB_NOT_FOUND` - Requested job doesn't exist
- `PROVER_UNAVAILABLE` - No workers available for task
- `SYSTEM_UNAVAILABLE` - System temporarily unavailable

## Protocol Testing with grpcurl

Install grpcurl for API testing:
```bash
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest
```

### Basic API Calls

**Note**: Coordinator defaults to port `8080`, Docker deployment uses `50051`.

```bash
# Health check
grpcurl -plaintext 127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/HealthCheck

# Service information  
grpcurl -plaintext 127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/StatusInfo

# System status
grpcurl -plaintext 127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/SystemStatus
```

### Job and Worker Management

```bash
# List active jobs
grpcurl -plaintext -d '{"active_only": true}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/JobsList

# List available workers
grpcurl -plaintext -d '{"available_only": true}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/ProversList

# Get specific job status
grpcurl -plaintext -d '{"job_id": "job_123"}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/JobStatus

# Launch a new proof job
grpcurl -plaintext -d '{"block_id": "block_456", "compute_capacity": 4, "input_path": "/path/to/input"}' \
  127.0.0.1:50051 zisk.distributed.api.v1.ZiskDistributedApi/LaunchProof
```

### Streaming API Testing

For testing the bidirectional `ProverStream`, you need a streaming-capable client. Example worker registration message:

```json
{
  "register": {
    "prover_id": "test-worker-001", 
    "compute_capacity": {"compute_units": 8}
  }
}
```

## Integration

### Rust Integration

Import this crate in coordinator or worker implementations:

```rust
use zisk_distributed_grpc_api::zisk_distributed_api::{
    ZiskDistributedApiServer, ZiskDistributedApiClient,
    ProverMessage, CoordinatorMessage,
    ExecuteTaskRequest, ExecuteTaskResponse,
    ProverRegisterRequest, ProverRegisterResponse,
    // ... other message types
};
```

### Client Implementation Example

```rust
use tonic::transport::Channel;
use zisk_distributed_grpc_api::zisk_distributed_api::{
    ZiskDistributedApiClient, ProverMessage, ProverRegisterRequest
};

async fn connect_worker() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://[::1]:50051")
        .connect()
        .await?;
    
    let mut client = ZiskDistributedApiClient::new(channel);
    
    let outbound = async_stream::stream! {
        yield ProverMessage {
            message: Some(proofman_message::Message::Register(ProverRegisterRequest {
                prover_id: Some("example-worker".to_string()),
                compute_capacity: Some(ComputeCapacity { 
                    compute_units: 4 
                }),
            })),
        };
    };
    
    let response = client.prover_stream(Request::new(outbound)).await?;
    let mut inbound = response.into_inner();
    
    while let Some(coordinator_message) = inbound.message().await? {
        // Handle coordinator messages
        println!("Received: {:?}", coordinator_message);
    }
    
    Ok(())
}
```

## Protocol Buffer Schema

The complete gRPC service definition is in `proto/zisk_distributed_api.proto` with detailed message schemas for all request/response types.

### Key Message Types

- `ProverMessage` / `CoordinatorMessage` - Top-level message envelopes
- `ExecuteTaskRequest` / `ExecuteTaskResponse` - Task execution protocol
- `ProverRegisterRequest` / `ProverRegisterResponse` - Worker registration
- `ComputeCapacity` - Worker capacity specification
- `JobStatus`, `SystemStatus` - Administrative data structures

For implementation details and deployment instructions, refer to the main [distributed README](../../README.md).
