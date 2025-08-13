# Consensus Client

A prover client for the Consensus Network that connects to the coordinator to participate in distributed proof generation.

## Installation

Build the client from the project root:

```bash
cargo build --bin consensus-client
```

Or run directly:

```bash
cargo run --bin consensus-client
```

## Configuration

The consensus client supports configuration through TOML files, with CLI arguments available to override configuration values.

### Configuration File

The client looks for a `config.toml` file by default, or you can specify a different path using the `--config` flag.

#### Generate Example Configuration

To generate an example configuration file:

```bash
cargo run --bin consensus-client -- --generate-config
```

This creates a `config.toml` file with default values and comments explaining each option.

#### Configuration Structure

```toml
[server]
# URL of the consensus server to connect to
url = "http://127.0.0.1:8080"

[prover]
# Prover ID (optional - if not set, a UUID will be auto-generated)
# prover_id = "my-prover-001"

[prover.compute_capacity]
# Number of compute units this prover can handle
# This represents the computational capacity of your prover
compute_units = 4

[connection]
# How often to attempt reconnection after a disconnect (in seconds)
reconnect_interval_seconds = 5

# Heartbeat timeout in seconds
heartbeat_timeout_seconds = 30
```

### Command Line Interface

All configuration values can be overridden via command line arguments:

```bash
# Show help and available options
cargo run --bin consensus-client -- --help

# Run with configuration file (default: config.toml)
cargo run --bin consensus-client

# Override server URL
cargo run --bin consensus-client -- --url "http://my-server:8080"

# Override prover ID
cargo run --bin consensus-client -- --prover-id "my-custom-prover"

# Override compute units
cargo run --bin consensus-client -- --compute-units 8

# Use custom configuration file
cargo run --bin consensus-client -- --config /path/to/my-config.toml

# Combine multiple overrides
cargo run --bin consensus-client -- \
  --url "http://production-server:8080" \
  --compute-units 16 \
  --prover-id "production-prover-001"
```

### Configuration Priority

Configuration values are loaded with the following priority (highest to lowest):

1. **Command line arguments** - Override everything else
2. **Configuration file** - Loaded from file specified by `--config` or `config.toml`
3. **Default values** - Built-in defaults if no config file exists

### Environment Variables

You can also specify the configuration file path using an environment variable:

```bash
export CONFIG_PATH="/path/to/my-config.toml"
cargo run --bin consensus-client
```

## Usage Examples

### Basic Usage

1. Generate a configuration file:
   ```bash
   cargo run --bin consensus-client -- --generate-config
   ```

2. Edit the `config.toml` file to match your setup:
   ```toml
   [server]
   url = "http://your-coordinator:8080"
   
   [prover.compute_capacity]
   compute_units = 8  # Adjust based on your hardware
   ```

3. Run the client:
   ```bash
   cargo run --bin consensus-client
   ```

### Development Setup

For development, you might want to quickly test with different configurations:

```bash
# Quick test with different server
cargo run --bin consensus-client -- --url "http://localhost:3030"

# Test with specific prover ID for debugging
cargo run --bin consensus-client -- --prover-id "debug-prover"

# Test with higher compute capacity
cargo run --bin consensus-client -- --compute-units 32
```

### Production Setup

For production environments:

1. Create a production configuration file:
   ```bash
   cargo run --bin consensus-client -- --config production.toml --generate-config
   ```

2. Edit `production.toml` with production values:
   ```toml
   [server]
   url = "https://consensus-coordinator.example.com:443"
   
   [prover]
   prover_id = "prod-prover-node-001"
   
   [prover.compute_capacity]
   compute_units = 64
   
   [connection]
   reconnect_interval_seconds = 10
   heartbeat_timeout_seconds = 60
   ```

3. Run with production config:
   ```bash
   cargo run --bin consensus-client --release -- --config production.toml
   ```

## Configuration Reference

| Configuration Key | CLI Argument | Type | Default | Description |
|------------------|--------------|------|---------|-------------|
| `server.url` | `--url` | String | `http://127.0.0.1:8080` | Coordinator server URL |
| `prover.prover_id` | `--prover-id` | String | Auto-generated UUID | Unique prover identifier |
| `prover.compute_capacity.compute_units` | `--compute-units` | Number | CPU count | Available compute units |
| `connection.reconnect_interval_seconds` | - | Number | 5 | Reconnection interval in seconds |
| `connection.heartbeat_timeout_seconds` | - | Number | 30 | Heartbeat timeout in seconds |

## Error Handling

If the configuration file is not found, the client will use default values and log a warning. If required values are missing (like server URL), the client will exit with an error message.

To troubleshoot configuration issues:

1. Use `--generate-config` to create a valid example
2. Check that the configuration file path is correct
3. Verify TOML syntax is valid
4. Use CLI overrides to test specific values
