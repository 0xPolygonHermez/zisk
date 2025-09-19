# Zisk Distributed System

A distributed proof generation system for the Zisk zkVM, consisting of a coordinator and multiple prover nodes.

## Architecture

This project is organized as a Rust workspace with multiple crates:

```
crates/
├── coordinator/          # Main coordinator service managing proof tasks
├── prover/              # Prover nodes that generate proofs
├── grpc-api/            # gRPC API definitions for communication
└── common/              # Shared functionality between components
```

## Docker Deployment

The distributed system can be deployed using Docker Compose:

```bash
# Build and start the services
docker-compose up -d

# View logs
docker-compose logs -f

# Scale prover nodes
docker-compose up -d --scale prover=3

# Stop the services
docker-compose down
```

## Command Line Usage

```bash
# Build the binaries
cargo build --release --bin coordinator --bin prover

# Run coordinator (from workspace root)
cargo run --bin coordinator

# Run prover (from workspace root) 
cargo run --bin prover

# With custom configuration
CONFIG_PATH=/path/to/config cargo run --bin coordinator
```

## Configuration

Configuration is loaded from TOML files with environment variable overrides:

### Environment Variables

```bash
# Service Configuration
DISTRIBUTED_SERVICE_NAME="zisk-distributed"
DISTRIBUTED_SERVICE_ENVIRONMENT="production"

# Server Configuration (Coordinator)
DISTRIBUTED_SERVER_HOST="0.0.0.0"
DISTRIBUTED_SERVER_PORT=50051

# Prover Configuration
DISTRIBUTED_SERVER_URL="http://coordinator:50051"
DISTRIBUTED_PROVER_ID="prover-001"

# Logging Configuration
DISTRIBUTED_LOGGING_LEVEL="info"
DISTRIBUTED_LOGGING_FORMAT="json"  # json, pretty, compact
DISTRIBUTED_LOGGING_FILE_OUTPUT=true
DISTRIBUTED_LOGGING_FILE_PATH="/var/log/distributed/app.log"
```

### Configuration Files

- `crates/coordinator/config/default.toml` - Coordinator default configuration
- `crates/coordinator/config/production.toml` - Production overrides
- `crates/prover/config/config.toml` - Prover configuration

## Development

### Prerequisites

- Rust 1.75+
- Docker & Docker Compose (for containerized deployment)

### Running the Services

```bash
# Development mode - coordinator
DISTRIBUTED_LOGGING_FORMAT=pretty cargo run --bin coordinator

# Development mode - prover
DISTRIBUTED_SERVER_URL=http://127.0.0.1:50051 cargo run --bin prover

# Production mode with JSON logging
DISTRIBUTED_LOGGING_FORMAT=json DISTRIBUTED_LOGGING_LEVEL=info cargo run --bin coordinator --release
```
## Docker

The system is containerized for easy deployment:

```bash
# Build from workspace root
docker build -f distributed/Dockerfile -t zisk-distributed .

# Run coordinator
docker run -p 50051:50051 -e DISTRIBUTED_LOGGING_LEVEL=info zisk-distributed coordinator

# Run prover
docker run -e DISTRIBUTED_SERVER_URL=http://coordinator:50051 zisk-distributed prover

# Using Docker Compose (recommended)
cd distributed/
docker-compose up -d

# Scale provers
docker-compose up -d --scale prover=5
```

## License

Apache-2.0 OR MIT
