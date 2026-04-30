#!/usr/bin/env bash
# install.sh — install zisk-coordinator as a systemd service on Linux.
#
# Usage:
#   sudo ./install.sh [OPTIONS]
#
# Options:
#   --env PATH       Load env vars from PATH (default: ./.env if present)
#   --binary PATH    Use a pre-built binary instead of building from source
#   --config PATH    Install an existing coordinator.toml instead of the sample
#   --port N         Listening port (default: 7000)
#   --no-start       Enable but do not start the service
#   --no-enable      Install unit file but do not enable or start
#   --uninstall      Stop, disable, and remove the service (prompts for cleanup)
#   -y, --yes        Skip every uninstall prompt (assume yes)
#
# Env-var equivalents (CLI flags win): ZISK_COORDINATOR_BINARY,
# ZISK_COORDINATOR_CONFIG, ZISK_COORDINATOR_PORT.
#
# What this script does:
#   1. Verifies it's running on Linux
#   2. Creates the 'zisk-coordinator' system user
#   3. Builds or installs the binary to /usr/local/bin/zisk-coordinator
#   4. Installs config to /etc/zisk/coordinator.toml
#   5. Creates the /var/lib/zisk working directory
#   6. Writes /etc/systemd/system/zisk-coordinator.service
#   7. Runs: systemctl daemon-reload && systemctl enable --now zisk-coordinator

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_DIR="${SCRIPT_DIR}/../common"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

# shellcheck source=../common/lib.sh
source "${COMMON_DIR}/lib.sh"
# shellcheck source=./defaults.env
source "${SCRIPT_DIR}/defaults.env"

require_os "Linux"

# ── load .env (if any), then argument parsing ─────────────────────────────────

load_env_file "$@"

BINARY_SRC="${ZISK_COORDINATOR_BINARY:-}"
CONFIG_SRC="${ZISK_COORDINATOR_CONFIG:-}"
PORT="${ZISK_COORDINATOR_PORT:-$DEFAULT_PORT}"
NO_START=false
NO_ENABLE=false
UNINSTALL=false
ASSUME_YES=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --env)       shift 2 ;;     # already consumed by load_env_file
        --binary)    BINARY_SRC="$2";  shift 2 ;;
        --config)    CONFIG_SRC="$2";  shift 2 ;;
        --port)      PORT="$2";        shift 2 ;;
        --no-start)  NO_START=true;    shift ;;
        --no-enable) NO_ENABLE=true;   shift ;;
        --uninstall) UNINSTALL=true;   shift ;;
        -y|--yes)    ASSUME_YES=true;  shift ;;
        *) die "Unknown option: $1" ;;
    esac
done

# ── uninstall ─────────────────────────────────────────────────────────────────

if $UNINSTALL; then
    need_root
    [[ -f "${UNIT_FILE}" ]] || die "${BINARY_NAME} is not installed (${UNIT_FILE} not found)."
    confirm "Uninstall ${BINARY_NAME}?" || { info "Cancelled."; exit 0; }

    # Recover install-time paths from unit metadata; fall back to defaults.env
    data_dir="$(read_unit_metadata "${UNIT_FILE}" "${BINARY_NAME}" DATA_DIR)"
    log_dir="$(read_unit_metadata "${UNIT_FILE}" "${BINARY_NAME}" LOG_DIR)"
    config_dir="$(read_unit_metadata "${UNIT_FILE}" "${BINARY_NAME}" CONFIG_DIR)"
    svc_user="$(read_unit_metadata "${UNIT_FILE}" "${BINARY_NAME}" SVC_USER)"
    svc_group="$(read_unit_metadata "${UNIT_FILE}" "${BINARY_NAME}" SVC_GROUP)"
    : "${data_dir:=$WORK_DIR}"
    : "${log_dir:=$LOG_DIR}"
    : "${config_dir:=$CONFIG_DIR}"
    : "${svc_user:=$SERVICE_USER}"
    : "${svc_group:=$SERVICE_GROUP}"

    info "Stopping and removing ${BINARY_NAME}..."
    systemctl stop    "${BINARY_NAME}" 2>/dev/null || true
    systemctl disable "${BINARY_NAME}" 2>/dev/null || true
    rm -f "${UNIT_FILE}" "${BINARY_DST}"
    systemctl daemon-reload

    prompt_remove_dir "${log_dir}" "log directory"
    prompt_remove_dir "${data_dir}" "data directory"
    prompt_remove_dir "${config_dir}" "config directory"
    prompt_remove_user_group "${svc_user}" "${svc_group}"

    info "${BINARY_NAME} uninstalled."
    exit 0
fi

# ── install ───────────────────────────────────────────────────────────────────

need_root

# 1. Build or use pre-built binary
build_or_use_binary "zisk-coordinator-server"

# 2. Create system group + user
if ! getent group "${SERVICE_GROUP}" &>/dev/null; then
    info "Creating system group '${SERVICE_GROUP}'..."
    groupadd --system "${SERVICE_GROUP}"
fi
if ! id "${SERVICE_USER}" &>/dev/null; then
    info "Creating system user '${SERVICE_USER}'..."
    useradd --system --gid "${SERVICE_GROUP}" --no-create-home --shell /usr/sbin/nologin "${SERVICE_USER}"
fi

# 3. Install binary
info "Installing binary to ${BINARY_DST}..."
install -m 755 "${BINARY_SRC}" "${BINARY_DST}"

# 4. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${SERVICE_GROUP}" \
    "${WORKSPACE_ROOT}/distributed/crates/coordinator-server/config/coordinator.example.toml"

# 5. Create working directory
mkdir -p "${WORK_DIR}"
chown "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}"

# 6. Write systemd unit file
info "Writing unit file to ${UNIT_FILE}..."
cat > "${UNIT_FILE}" <<EOF
[Unit]
Description=ZisK Coordinator — coordinator server for the ZisK proving system
Documentation=https://github.com/0xPolygonHermez/zisk
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${SERVICE_USER}
Group=${SERVICE_GROUP}
WorkingDirectory=${WORK_DIR}
ExecStart=${BINARY_DST} --config ${CONFIG_DST} -p ${PORT}
Restart=on-failure
RestartSec=3
LimitNOFILE=65535
Nice=-10
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

# Install metadata (read by --uninstall; do not edit)
# ${BINARY_NAME}:DATA_DIR=${WORK_DIR}
# ${BINARY_NAME}:LOG_DIR=${LOG_DIR}
# ${BINARY_NAME}:CONFIG_DIR=${CONFIG_DIR}
# ${BINARY_NAME}:SVC_USER=${SERVICE_USER}
# ${BINARY_NAME}:SVC_GROUP=${SERVICE_GROUP}
EOF

systemctl daemon-reload

if $NO_ENABLE; then
    info "Unit file installed. Service not enabled (--no-enable)."
    info "To enable: sudo systemctl enable --now ${BINARY_NAME}"
elif $NO_START; then
    systemctl enable "${BINARY_NAME}"
    info "Service enabled but not started (--no-start)."
    info "To start: sudo systemctl start ${BINARY_NAME}"
else
    systemctl enable --now "${BINARY_NAME}"
    echo
    info "✓ ${BINARY_NAME} installed and started."
    echo
    echo "  Status:    systemctl status ${BINARY_NAME}"
    echo "  Logs:      journalctl -u ${BINARY_NAME} -f"
    echo "  Restart:   systemctl restart ${BINARY_NAME}"
    echo "  Uninstall: sudo $(basename "${BASH_SOURCE[0]}") --uninstall"
fi
