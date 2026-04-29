#!/usr/bin/env bash
# install.sh — install zisk-coordinator as a systemd service on Linux.
#
# Usage:
#   sudo ./scripts/install.sh [OPTIONS]
#
# Options:
#   --binary PATH    Use a pre-built binary instead of building from source
#   --config PATH    Install an existing coordinator.toml instead of the sample
#   --no-start       Enable but do not start the service
#   --no-enable      Install unit file but do not enable or start
#   --uninstall      Stop, disable, and remove the service and binary
#
# What this script does:
#   1. Creates the 'zisk-coordinator' system user
#   2. Installs the binary to /usr/local/bin/zisk-coordinator
#   3. Installs config to /etc/zisk/coordinator.toml
#   4. Creates the /var/lib/zisk working directory
#   5. Writes /etc/systemd/system/zisk-coordinator.service
#   6. Runs: systemctl daemon-reload && systemctl enable --now zisk-coordinator

set -euo pipefail

BINARY_NAME="zisk-coordinator"
BINARY_DST="/usr/local/bin/${BINARY_NAME}"
CONFIG_DIR="/etc/zisk"
CONFIG_DST="${CONFIG_DIR}/coordinator.toml"
WORK_DIR="/var/lib/zisk"
UNIT_FILE="/etc/systemd/system/${BINARY_NAME}.service"
SERVICE_USER="${BINARY_NAME}"

# ── argument parsing ──────────────────────────────────────────────────────────

BINARY_SRC=""
CONFIG_SRC=""
NO_START=false
NO_ENABLE=false
UNINSTALL=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --binary)   BINARY_SRC="$2"; shift 2 ;;
        --config)   CONFIG_SRC="$2"; shift 2 ;;
        --no-start) NO_START=true;  shift ;;
        --no-enable) NO_ENABLE=true; shift ;;
        --uninstall) UNINSTALL=true; shift ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# ── helpers ───────────────────────────────────────────────────────────────────

need_root() {
    if [[ $EUID -ne 0 ]]; then
        echo "Error: this script must be run as root (sudo)." >&2
        exit 1
    fi
}

# ── uninstall ─────────────────────────────────────────────────────────────────

if $UNINSTALL; then
    need_root
    echo "Uninstalling ${BINARY_NAME}..."
    systemctl stop    "${BINARY_NAME}" 2>/dev/null || true
    systemctl disable "${BINARY_NAME}" 2>/dev/null || true
    rm -f "${UNIT_FILE}" "${BINARY_DST}"
    systemctl daemon-reload
    echo "Done. Config files under ${CONFIG_DIR}/ are left in place."
    echo "Remove manually if no longer needed: sudo rm -rf ${CONFIG_DIR}"
    exit 0
fi

# ── install ───────────────────────────────────────────────────────────────────

need_root

# 1. Build or use pre-built binary
if [[ -z "${BINARY_SRC}" ]]; then
    echo "Building ${BINARY_NAME} from source..."
    # Resolve workspace root (two levels up from scripts/)
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
    cargo build --release -p zisk-coordinator-server --manifest-path "${WORKSPACE_ROOT}/Cargo.toml"
    BINARY_SRC="${WORKSPACE_ROOT}/target/release/${BINARY_NAME}"
fi

if [[ ! -f "${BINARY_SRC}" ]]; then
    echo "Error: binary not found at ${BINARY_SRC}" >&2
    exit 1
fi

# 2. Create system user
if ! id "${SERVICE_USER}" &>/dev/null; then
    echo "Creating system user '${SERVICE_USER}'..."
    useradd --system --no-create-home --shell /usr/sbin/nologin "${SERVICE_USER}"
fi

# 3. Install binary
echo "Installing binary to ${BINARY_DST}..."
install -m 755 "${BINARY_SRC}" "${BINARY_DST}"

# 4. Install config
mkdir -p "${CONFIG_DIR}"
if [[ -n "${CONFIG_SRC}" ]]; then
    echo "Installing config from ${CONFIG_SRC}..."
    install -m 640 -o root -g "${SERVICE_USER}" "${CONFIG_SRC}" "${CONFIG_DST}"
elif [[ ! -f "${CONFIG_DST}" ]]; then
    echo "Installing sample config to ${CONFIG_DST}..."
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    SAMPLE="${SCRIPT_DIR}/../config/coordinator.example.toml"
    if [[ -f "${SAMPLE}" ]]; then
        install -m 640 -o root -g "${SERVICE_USER}" "${SAMPLE}" "${CONFIG_DST}"
    else
        echo "Warning: sample config not found at ${SAMPLE}; skipping." >&2
    fi
else
    echo "Config already exists at ${CONFIG_DST}; leaving unchanged."
fi

# 5. Create working directory
mkdir -p "${WORK_DIR}"
chown "${SERVICE_USER}:${SERVICE_USER}" "${WORK_DIR}"

# 6. Write systemd unit file
echo "Writing unit file to ${UNIT_FILE}..."
cat > "${UNIT_FILE}" <<EOF
[Unit]
Description=ZisK Coordinator — coordinator server for the ZisK proving system
Documentation=https://github.com/0xPolygonHermez/zisk
After=network.target

[Service]
Type=simple
User=${SERVICE_USER}
Group=${SERVICE_USER}
ExecStart=${BINARY_DST} --config ${CONFIG_DST}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=${BINARY_NAME}

# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${WORK_DIR}
ReadOnlyPaths=${CONFIG_DIR}

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload

if $NO_ENABLE; then
    echo "Unit file installed. Service not enabled (--no-enable)."
    echo "To enable: sudo systemctl enable --now ${BINARY_NAME}"
elif $NO_START; then
    systemctl enable "${BINARY_NAME}"
    echo "Service enabled but not started (--no-start)."
    echo "To start: sudo systemctl start ${BINARY_NAME}"
else
    systemctl enable --now "${BINARY_NAME}"
    echo ""
    echo "✓ ${BINARY_NAME} installed and started."
    echo ""
    echo "  Status:    systemctl status ${BINARY_NAME}"
    echo "  Logs:      journalctl -u ${BINARY_NAME} -f"
    echo "  Restart:   systemctl restart ${BINARY_NAME}"
    echo "  Uninstall: sudo $(basename "${BASH_SOURCE[0]}") --uninstall"
fi
