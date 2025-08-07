# Consensus Network Client

A command-line client for interacting with Consensus Network services.

## Installation

Build the client from the project root:

```bash
cargo build --bin consensus-client
```

Or run directly:

```bash
cargo run --bin consensus-client -- [COMMAND]
```

## Usage

### Basic Commands

```bash
# Get server status
consensus-client status

# Check server health
consensus-client health

# Get server info
consensus-client info

# List connected peers
consensus-client peers

# Submit a task
consensus-client task --task-type "compute" --payload '{"data": "value"}' --priority 5

# Get task status by ID
consensus-client task-status [TASK_ID]

# Interactive mode
consensus-client interactive
```

### Server Configuration

By default, the client connects to `http://127.0.0.1:8080`. You can specify a different server:

```bash
consensus-client --url http://localhost:3030 status
```

### Interactive Mode

The interactive mode provides a convenient shell for running multiple commands:

```bash
consensus-client interactive
```

Once in interactive mode, you can use these commands:
- `status` - Get server status
- `info` - Get server information  
- `health` - Check server health
- `peers` - List connected peers
- `task` - Submit a new task (guided input)
- `help` - Show available commands
- `quit` - Exit interactive mode

## Examples

### Submit a computation task
```bash
consensus-client task \
  --task-type "computation" \
  --payload '{"algorithm": "factorial", "input": 10}' \
  --priority 5
```

### Check server health with custom URL
```bash
consensus-client --url http://production-server:8080 health
```

### Interactive session
```bash
$ consensus-client interactive
🚀 Consensus Network Interactive Client
Connected to: http://127.0.0.1:8080
Commands: status, info, health, peers, task, quit

consensus> status
{
  "health": "Healthy",
  "uptime_seconds": 42,
  ...
}

consensus> quit
👋 Goodbye!
```

## Error Handling

The client provides detailed error messages for common issues:
- Connection failures
- Invalid JSON payloads
- Server errors
- Network timeouts

## Output Format

All responses are formatted as pretty-printed JSON for easy reading. The client also includes emoji indicators for different command types:

- 📊 Server Status
- 🏥 Health Check  
- ℹ️ Server Info
- 👥 Connected Peers
- 📋 Task Operations
