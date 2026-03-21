# zisk-node

The ZisK node daemon (`zisklet`) — an API gateway that exposes the ZisK proof submission API to clients and routes requests to the configured coordinator.

## Overview

`zisklet` serves `ZiskUserApi` (`zisk.user.v1`) on the configured port — a gRPC API for client applications submitting proof jobs and managing guest programs.

The cluster topology (coordinator address, workers) is loaded from `clusters.yml` at startup. Exactly one cluster is supported.

## Building

```bash
cargo build --release -p zisk-node
# binary at: target/release/zisklet
```

## Configuration

### `node.toml`

```toml
[service]
name = "ZisK Node"

[server]
host = "0.0.0.0"
port = 7000                    # gRPC listen port
shutdown_timeout_seconds = 30

[logging]
level = "info"                 # trace | debug | info | warn | error
format = "pretty"

[node]
clusters_file = "/etc/zisk/clusters.yml"
# advertise_addr = "192.168.1.10:7000"   # address peers use to reach this node
work_dir = "/var/lib/zisk"
```

The config file path can be overridden with `--config` or `ZISK_NODE_CONFIG`. The port can be overridden with `--port` or `ZISK_NODE_PORT`.

#### Logging

`logging.level` sets the minimum log level (`trace` | `debug` | `info` | `warn` | `error`). The `RUST_LOG` environment variable takes precedence over the config file value and supports per-module filters (e.g. `RUST_LOG=zisk_node=debug,info`).

`logging.format` controls output format:

| Value    | Output                              | When to use                        |
|----------|-------------------------------------|------------------------------------|
| `pretty` | Human-readable, coloured (default)  | Local development and debugging    |
| `json`   | One JSON object per line            | Production — ship to log aggregators (Loki, Datadog, CloudWatch) |

Every gRPC call is logged at `INFO` with method, status code, and latency. Requests exceeding 5 seconds are additionally logged at `WARN`. Server-side errors (`Internal`, `DataLoss`, `Unknown`) are logged at `ERROR`.

### `clusters.yml`

Exactly one cluster must be defined. Each cluster has one coordinator and zero or more workers. Nodes listed under `nodes` are used to resolve coordinator and worker addresses.

```yaml
nodes:
  node-0:
    address: "10.0.0.1"
    port: 7000
    gpus:
      - id: 0
        memory_gb: 40
  node-1:
    address: "10.0.0.2"
    port: 7000
    gpus:
      - id: 0
        memory_gb: 40

clusters:
  default:
    coordinator:
      node: node-0
      instance: coordinator-0
      port: 50000
    workers:
      - node: node-0
        instance: worker-0
        port: 50001
        gpus: [0]
      - node: node-1
        instance: worker-1
        port: 50001
        gpus: [0]
```

## Running with Docker

### Quick start

```bash
# From the workspace root:
docker compose -f node/docker/docker-compose.yml up
```

This builds the image from source, mounts `docker/config/node.toml` and `docker/config/clusters.yml`, and starts `zisklet` on port `7000`. Edit those files before starting to match your environment.

### Build the image manually

```bash
# From the workspace root:
docker build -f node/docker/Dockerfile -t zisklet .
```

### Run the container manually

```bash
docker run \
  -v ./node/docker/config:/etc/zisk:ro \
  -p 7000:7000 \
  zisklet
```

### Notes

- The build context is the **workspace root**, not the crate directory — this is required so `cargo` can resolve the full workspace.
- The `docker/config/` directory contains sample files for local development. In production, supply your own `node.toml` and `clusters.yml` via volume mounts or a secret manager.
- The container runs as an unprivileged `zisklet` system user.

---

## Installation

### Automated (recommended)

The install script builds the binary, creates a dedicated system user, installs config files, and registers a systemd service:

```bash
sudo ./scripts/install.sh
```

**Options:**

| Flag | Description |
|------|-------------|
| `--binary PATH` | Use a pre-built binary instead of building from source |
| `--config PATH` | Install an existing `node.toml` instead of writing a sample |
| `--clusters PATH` | Install an existing `clusters.yml` instead of writing a sample |
| `--no-start` | Enable but do not start the service |
| `--no-enable` | Install unit file but do not enable or start |
| `--uninstall` | Stop, disable, and remove the service and binary |

**Example — use a pre-built binary and custom configs:**
```bash
sudo ./scripts/install.sh \
  --binary ./target/release/zisklet \
  --config /path/to/node.toml \
  --clusters /path/to/clusters.yml
```

### Manual

1. Copy the binary:
   ```bash
   sudo install -m 755 target/release/zisklet /usr/local/bin/zisklet
   ```

2. Create a system user:
   ```bash
   sudo useradd --system --no-create-home --shell /usr/sbin/nologin zisklet
   ```

3. Install config files:
   ```bash
   sudo mkdir -p /etc/zisk
   sudo cp node.toml /etc/zisk/node.toml
   sudo cp clusters.yml /etc/zisk/clusters.yml
   sudo chown root:zisklet /etc/zisk/node.toml /etc/zisk/clusters.yml
   sudo chmod 640 /etc/zisk/node.toml /etc/zisk/clusters.yml
   ```

4. Create the systemd unit file at `/etc/systemd/system/zisklet.service`:
   ```ini
   [Unit]
   Description=ZisK Node Daemon
   After=network.target

   [Service]
   Type=simple
   User=zisklet
   Group=zisklet
   ExecStart=/usr/local/bin/zisklet --config /etc/zisk/node.toml
   Restart=on-failure
   RestartSec=5
   StandardOutput=journal
   StandardError=journal
   SyslogIdentifier=zisklet
   NoNewPrivileges=true
   PrivateTmp=true
   ProtectSystem=strict
   ReadWritePaths=/var/lib/zisk
   ReadOnlyPaths=/etc/zisk

   [Install]
   WantedBy=multi-user.target
   ```

5. Enable and start:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now zisklet
   ```

## Managing the service

```bash
# Status
systemctl status zisklet

# Live logs
journalctl -u zisklet -f

# Restart after config change
systemctl restart zisklet

# Stop
systemctl stop zisklet

# Uninstall
sudo ./scripts/install.sh --uninstall
```

## Testing

```bash
# Standard runner
cargo test -p zisk-node

# cargo-nextest (faster, better output — recommended)
cargo nextest run -p zisk-node

# CI profile (4 threads, 1 retry, fail-fast)
cargo nextest run -p zisk-node --profile ci
```

Install nextest once with:
```bash
cargo install cargo-nextest --locked
```

## Uninstalling

```bash
sudo ./scripts/install.sh --uninstall
```

Config files under `/etc/zisk/` are left in place and must be removed manually if no longer needed.
