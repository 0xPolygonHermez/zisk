# gRPC Server Usage

## Available RPC Methods

1. **GetInfo()** - Get service information
2. **HealthCheck()** - Basic health check  
3. **GetStatus()** - Detailed service status (not implemented in demo)
4. **ListPeers()** - List connected peers (not implemented in demo)
5. **SubmitTask(task)** - Submit new task (not implemented in demo)
6. **GetTask(task_id)** - Get task status (not implemented in demo)

## Testing with grpcurl

You can test the server using `grpcurl`:

```bash
# Install grpcurl if not already installed
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# Test the GetInfo endpoint
grpcurl -plaintext 127.0.0.1:50051 consensus.api.v1.ConsensusApi/GetInfo

# Test the HealthCheck endpoint  
grpcurl -plaintext 127.0.0.1:50051 consensus.api.v1.ConsensusApi/HealthCheck
```

## Protocol Buffer Schema

The full gRPC service definition is available in `proto/distributed_api.proto`.
