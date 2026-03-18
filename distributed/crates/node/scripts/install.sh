#!/usr/bin/env bash
# install.sh — install or uninstall the zisklet daemon as a systemd service.
#
# Usage:
#   sudo ./scripts/install.sh [OPTIONS]
#
# Options:
#   --binary PATH       Path to pre-built zisklet binary (default: build from source)
#   --config PATH       Path to existing node.toml to install (default: write sample)
#   --clusters PATH     Path to existing clusters.yml to install (default: write sample)
#   --no-start          Install and enable but do not start the service
#   --no-enable         Install but do not enable or start the service
#   --uninstall         Stop, disable, and remove zisklet from this machine
#   -h, --help          Show this message

set -euo pipefail

# ─── Defaults ────────────────────────────────────────────────────────────────

BINARY_SRC=""
CONFIG_SRC=""
CLUSTERS_SRC=""
NO_START=false
NO_ENABLE=false
UNINSTALL=false

INSTALL_BIN="/usr/local/bin/zisklet"
INSTALL_CONFIG_DIR="/etc/zisk"
INSTALL_CONFIG="$INSTALL_CONFIG_DIR/node.toml"
INSTALL_CLUSTERS="$INSTALL_CONFIG_DIR/clusters.yml"
UNIT_FILE="/etc/systemd/system/zisklet.service"
UNIT_NAME="zisklet.service"
RUN_USER="zisklet"

# ─── Argument parsing ─────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)   BINARY_SRC="$2";   shift 2 ;;
    --config)   CONFIG_SRC="$2";   shift 2 ;;
    --clusters) CLUSTERS_SRC="$2"; shift 2 ;;
    --no-start)  NO_START=true;    shift ;;
    --no-enable) NO_ENABLE=true;   shift ;;
    --uninstall) UNINSTALL=true;   shift ;;
    -h|--help)
      sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# ─── Root check ───────────────────────────────────────────────────────────────

if [[ $EUID -ne 0 ]]; then
  echo "error: this script must be run as root (sudo $0 $*)" >&2
  exit 1
fi

# ─── Uninstall ────────────────────────────────────────────────────────────────

if $UNINSTALL; then
  echo "Stopping and disabling zisklet..."
  systemctl stop "$UNIT_NAME"  2>/dev/null || true
  systemctl disable "$UNIT_NAME" 2>/dev/null || true
  rm -f "$UNIT_FILE"
  systemctl daemon-reload
  rm -f "$INSTALL_BIN"
  echo "zisklet uninstalled."
  echo "Config files left in place: $INSTALL_CONFIG_DIR"
  exit 0
fi

# ─── Build binary if not provided ─────────────────────────────────────────────

if [[ -z "$BINARY_SRC" ]]; then
  echo "Building zisklet from source..."
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
  WORKSPACE_DIR="$(cd "$CRATE_DIR/../../../.." && pwd)"

  cargo build --release -p zisk-distributed-node \
    --manifest-path "$WORKSPACE_DIR/Cargo.toml"

  BINARY_SRC="$WORKSPACE_DIR/target/release/zisklet"
fi

if [[ ! -f "$BINARY_SRC" ]]; then
  echo "error: binary not found at '$BINARY_SRC'" >&2
  exit 1
fi

# ─── Install binary ───────────────────────────────────────────────────────────

echo "Installing binary to $INSTALL_BIN..."
install -m 755 "$BINARY_SRC" "$INSTALL_BIN"

# ─── Create system user ───────────────────────────────────────────────────────

if ! id "$RUN_USER" &>/dev/null; then
  echo "Creating system user '$RUN_USER'..."
  useradd --system --no-create-home --shell /usr/sbin/nologin "$RUN_USER"
fi

# ─── Install config ───────────────────────────────────────────────────────────

mkdir -p "$INSTALL_CONFIG_DIR"

if [[ -n "$CONFIG_SRC" ]]; then
  echo "Installing config from $CONFIG_SRC..."
  install -m 640 -o root -g "$RUN_USER" "$CONFIG_SRC" "$INSTALL_CONFIG"
elif [[ ! -f "$INSTALL_CONFIG" ]]; then
  echo "Writing sample config to $INSTALL_CONFIG..."
  cat > "$INSTALL_CONFIG" <<'EOF'
[service]
name = "ZisK Node"

[server]
host = "0.0.0.0"
port = 7000
shutdown_timeout_seconds = 30

[logging]
level = "info"
format = "pretty"

[node]
# Path to clusters.yml (required for cluster routing)
clusters_file = "/etc/zisk/clusters.yml"
# Advertised address for this node (used by peers)
# advertise_addr = "192.168.1.10:7000"
work_dir = "/var/lib/zisk"
EOF
  chown root:"$RUN_USER" "$INSTALL_CONFIG"
  chmod 640 "$INSTALL_CONFIG"
fi

# ─── Install clusters.yml ─────────────────────────────────────────────────────

if [[ -n "$CLUSTERS_SRC" ]]; then
  echo "Installing clusters.yml from $CLUSTERS_SRC..."
  install -m 640 -o root -g "$RUN_USER" "$CLUSTERS_SRC" "$INSTALL_CLUSTERS"
elif [[ ! -f "$INSTALL_CLUSTERS" ]]; then
  echo "Writing sample clusters.yml to $INSTALL_CLUSTERS..."
  cat > "$INSTALL_CLUSTERS" <<'EOF'
# clusters.yml — cluster topology for this zisklet node.
# Exactly one cluster must be defined.
#
# machines: named machines with their node address and available GPUs.
# clusters: defines one coordinator and its workers.

machines:
  this-machine:
    node: "127.0.0.1"
    port: 7000
    gpus:
      - id: 0
        memory_gb: 40

clusters:
  default:
    coordinator:
      machine: this-machine
      instance: coordinator-0
      port: 50000
    workers:
      - machine: this-machine
        instance: worker-0
        port: 50001
        gpus: [0]
EOF
  chown root:"$RUN_USER" "$INSTALL_CLUSTERS"
  chmod 640 "$INSTALL_CLUSTERS"
fi

# ─── Create working directory ─────────────────────────────────────────────────

mkdir -p /var/lib/zisk
chown "$RUN_USER":"$RUN_USER" /var/lib/zisk

# ─── Install systemd unit ─────────────────────────────────────────────────────

echo "Installing systemd unit to $UNIT_FILE..."
cat > "$UNIT_FILE" <<EOF
[Unit]
Description=ZisK Node Daemon
Documentation=https://github.com/0xPolygonHermez/zisk
After=network.target

[Service]
Type=simple
User=$RUN_USER
Group=$RUN_USER
ExecStart=$INSTALL_BIN --config $INSTALL_CONFIG
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=zisklet

# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/var/lib/zisk
ReadOnlyPaths=$INSTALL_CONFIG_DIR

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload

# ─── Enable / start ───────────────────────────────────────────────────────────

if ! $NO_ENABLE; then
  echo "Enabling zisklet..."
  systemctl enable "$UNIT_NAME"

  if ! $NO_START; then
    echo "Starting zisklet..."
    systemctl start "$UNIT_NAME"
    sleep 1
    systemctl status "$UNIT_NAME" --no-pager || true
  fi
fi

# ─── Done ─────────────────────────────────────────────────────────────────────

echo ""
echo "zisklet installed successfully."
echo ""
echo "  Binary:   $INSTALL_BIN"
echo "  Config:   $INSTALL_CONFIG"
echo "  Clusters: $INSTALL_CLUSTERS"
echo "  Unit:     $UNIT_FILE"
echo ""
echo "Useful commands:"
echo "  systemctl status zisklet"
echo "  journalctl -u zisklet -f"
echo "  systemctl restart zisklet"
echo "  sudo $0 --uninstall"
