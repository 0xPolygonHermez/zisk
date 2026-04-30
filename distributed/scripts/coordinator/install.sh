#!/usr/bin/env bash
# install.sh — install zisk-coordinator as a systemd service (Linux) or
#              launchd daemon (macOS).
#
# Usage:
#   sudo ./install.sh [OPTIONS]
#
# Options:
#   --env PATH         Load env vars from PATH (default: ./.env if present)
#   --binary PATH      Use a pre-built binary instead of building from source
#   --config PATH      Install an existing coordinator.toml instead of the sample
#   --api-port N       Client-facing gRPC API port (default: 7000)
#   --cluster-port N   Worker-facing gRPC port (optional; TOML default if unset)
#   --metrics-port N   Prometheus metrics port (optional; TOML default if unset)
#   --log-level LEVEL  trace | debug | info | warn | error (optional; RUST_LOG)
#   --no-start         Linux: enable but don't start.
#                      macOS: write plist but don't load (same as --no-enable).
#   --no-enable        Linux: install unit but don't enable or start.
#                      macOS: write plist but don't load.
#   --uninstall        Stop, disable, and remove the service (prompts for cleanup)
#   -y, --yes          Skip every uninstall prompt (assume yes)
#
# Env-var equivalents (CLI flags win): ZISK_COORDINATOR_BINARY,
# ZISK_COORDINATOR_CONFIG, ZISK_COORDINATOR_API_PORT,
# ZISK_COORDINATOR_CLUSTER_PORT, ZISK_COORDINATOR_METRICS_PORT, RUST_LOG.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_DIR="${SCRIPT_DIR}/../common"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

# shellcheck source=../common/lib.sh
source "${COMMON_DIR}/lib.sh"
# shellcheck source=./defaults.env
source "${SCRIPT_DIR}/defaults.env"

[[ "$OS_NAME" == "Linux" || "$OS_NAME" == "Darwin" ]] || \
    die "unsupported OS: ${OS_NAME}. Only Linux and Darwin are supported."

# ── load .env (if any), then argument parsing ─────────────────────────────────

load_env_file "$@"

BINARY_SRC="${ZISK_COORDINATOR_BINARY:-}"
CONFIG_SRC="${ZISK_COORDINATOR_CONFIG:-}"
API_PORT="${ZISK_COORDINATOR_API_PORT:-$DEFAULT_API_PORT}"
CLUSTER_PORT="${ZISK_COORDINATOR_CLUSTER_PORT:-}"
METRICS_PORT="${ZISK_COORDINATOR_METRICS_PORT:-}"
LOG_LEVEL="${RUST_LOG:-}"
NO_START=false
NO_ENABLE=false
UNINSTALL=false
ASSUME_YES=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --env)           shift 2 ;;     # already consumed by load_env_file
        --binary)        BINARY_SRC="$2";    shift 2 ;;
        --config)        CONFIG_SRC="$2";    shift 2 ;;
        --api-port)      API_PORT="$2";      shift 2 ;;
        --cluster-port)  CLUSTER_PORT="$2";  shift 2 ;;
        --metrics-port)  METRICS_PORT="$2";  shift 2 ;;
        --log-level)     LOG_LEVEL="$2";     shift 2 ;;
        --no-start)      NO_START=true;      shift ;;
        --no-enable)     NO_ENABLE=true;     shift ;;
        --uninstall)     UNINSTALL=true;     shift ;;
        -y|--yes)        ASSUME_YES=true;    shift ;;
        *) die "Unknown option: $1" ;;
    esac
done

# ── uninstall ─────────────────────────────────────────────────────────────────

if $UNINSTALL; then
    need_root

    if [[ "$OS_NAME" == "Darwin" ]]; then
        marker="${LAUNCHD_PLIST}"
    else
        marker="${UNIT_FILE}"
    fi
    [[ -f "$marker" ]] || die "${BINARY_NAME} is not installed (${marker} not found)."
    confirm "Uninstall ${BINARY_NAME}?" || { info "Cancelled."; exit 0; }

    # Recover install-time paths from metadata; fall back to defaults.env
    data_dir="$(read_unit_metadata "$marker" "${BINARY_NAME}" DATA_DIR)"
    log_dir="$(read_unit_metadata "$marker" "${BINARY_NAME}" LOG_DIR)"
    config_dir="$(read_unit_metadata "$marker" "${BINARY_NAME}" CONFIG_DIR)"
    svc_user="$(read_unit_metadata "$marker" "${BINARY_NAME}" SVC_USER)"
    svc_group="$(read_unit_metadata "$marker" "${BINARY_NAME}" SVC_GROUP)"
    : "${data_dir:=$WORK_DIR}"
    : "${log_dir:=$LOG_DIR}"
    : "${config_dir:=$CONFIG_DIR}"
    : "${svc_user:=$SERVICE_USER}"
    : "${svc_group:=$SERVICE_GROUP}"

    info "Stopping and removing ${BINARY_NAME}..."
    if [[ "$OS_NAME" == "Darwin" ]]; then
        launchctl unload "${LAUNCHD_PLIST}" 2>/dev/null || true
        rm -f "${LAUNCHD_PLIST}" "${BINARY_DST}" "${NEWSYSLOG_CONF}"
    else
        systemctl stop    "${BINARY_NAME}" 2>/dev/null || true
        systemctl disable "${BINARY_NAME}" 2>/dev/null || true
        rm -f "${UNIT_FILE}" "${BINARY_DST}"
        systemctl daemon-reload
    fi

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
create_service_user "${SERVICE_USER}" "${SERVICE_GROUP}" "ZisK Coordinator" "/var/empty"

# 3. Install binary
install_binary "${BINARY_SRC}" "${BINARY_DST}"

# 4. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${SERVICE_GROUP}" \
    "${WORKSPACE_ROOT}/distributed/crates/coordinator-server/config/coordinator.example.toml"

# 5. Create working (and log on macOS) directories
if [[ "$OS_NAME" == "Darwin" ]]; then
    mkdir -p "${WORK_DIR}" "${LOG_DIR}"
    chown "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}" "${LOG_DIR}"
else
    mkdir -p "${WORK_DIR}"
    chown "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}"
fi

# 6. Write service unit
if [[ "$OS_NAME" == "Darwin" ]]; then
    info "Writing plist to ${LAUNCHD_PLIST}..."

    build_program_args() {
        printf '        <string>%s</string>\n' "${BINARY_DST}"
        printf '        <string>--config</string>\n'
        printf '        <string>%s</string>\n' "${CONFIG_DST}"
        printf '        <string>--api-port</string>\n'
        printf '        <string>%s</string>\n' "${API_PORT}"
        if [[ -n "$CLUSTER_PORT" ]]; then
            printf '        <string>--cluster-port</string>\n'
            printf '        <string>%s</string>\n' "${CLUSTER_PORT}"
        fi
        if [[ -n "$METRICS_PORT" ]]; then
            printf '        <string>--metrics-port</string>\n'
            printf '        <string>%s</string>\n' "${METRICS_PORT}"
        fi
        if [[ -n "$LOG_LEVEL" ]]; then
            printf '        <string>--log-level</string>\n'
            printf '        <string>%s</string>\n' "${LOG_LEVEL}"
        fi
    }

    cat > "${LAUNCHD_PLIST}" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${LAUNCHD_LABEL}</string>

    <key>ProgramArguments</key>
    <array>
$(build_program_args)    </array>

    <key>UserName</key>
    <string>${SERVICE_USER}</string>

    <key>GroupName</key>
    <string>${SERVICE_GROUP}</string>

    <key>WorkingDirectory</key>
    <string>${WORK_DIR}</string>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>${LOG_DIR}/${BINARY_NAME}.log</string>

    <key>StandardErrorPath</key>
    <string>${LOG_DIR}/${BINARY_NAME}.log</string>

    <key>Nice</key>
    <integer>-10</integer>

    <key>SoftResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>65535</integer>
    </dict>
</dict>
</plist>

<!-- Install metadata (read by --uninstall; do not edit) -->
<!-- ${BINARY_NAME}:DATA_DIR=${WORK_DIR} -->
<!-- ${BINARY_NAME}:LOG_DIR=${LOG_DIR} -->
<!-- ${BINARY_NAME}:CONFIG_DIR=${CONFIG_DIR} -->
<!-- ${BINARY_NAME}:SVC_USER=${SERVICE_USER} -->
<!-- ${BINARY_NAME}:SVC_GROUP=${SERVICE_GROUP} -->
PLIST

    chown root:wheel "${LAUNCHD_PLIST}"
    chmod 0644 "${LAUNCHD_PLIST}"

    # 7. Write newsyslog rotation config
    info "Writing newsyslog config to ${NEWSYSLOG_CONF}..."
    cat > "${NEWSYSLOG_CONF}" <<NEWSYSLOG
# ${BINARY_NAME} log rotation — max ${LOG_MAX_SIZE_MB}MB per file, keep ${LOG_ROTATIONS} rotations, gzipped
${LOG_DIR}/${BINARY_NAME}.log  ${SERVICE_USER}:${SERVICE_GROUP}  640  ${LOG_ROTATIONS}  $(( LOG_MAX_SIZE_MB * 1024 ))  *  JG
NEWSYSLOG
    chmod 0644 "${NEWSYSLOG_CONF}"
else
    # Build ExecStart line — required flags first, optional appended only when set
    EXEC_START="ExecStart=${BINARY_DST} --config ${CONFIG_DST} --api-port ${API_PORT}"
    [[ -n "$CLUSTER_PORT" ]] && EXEC_START+=" --cluster-port ${CLUSTER_PORT}"
    [[ -n "$METRICS_PORT" ]] && EXEC_START+=" --metrics-port ${METRICS_PORT}"
    [[ -n "$LOG_LEVEL" ]]    && EXEC_START+=" --log-level ${LOG_LEVEL}"

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
${EXEC_START}
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
fi

# 8. Activate service (or skip)
SHOW_HINTS=true
if [[ "$OS_NAME" == "Darwin" ]]; then
    if $NO_ENABLE || $NO_START; then
        flag=$($NO_ENABLE && echo "--no-enable" || echo "--no-start")
        info "Plist installed. Service not loaded (${flag})."
        info "To load: sudo launchctl load -w ${LAUNCHD_PLIST}"
        SHOW_HINTS=false
    else
        info "Loading service via launchctl..."
        launchctl unload "${LAUNCHD_PLIST}" 2>/dev/null || true
        launchctl load -w "${LAUNCHD_PLIST}"
    fi
else
    systemctl daemon-reload
    if $NO_ENABLE; then
        info "Unit file installed. Service not enabled (--no-enable)."
        info "To enable: sudo systemctl enable --now ${BINARY_NAME}"
        SHOW_HINTS=false
    elif $NO_START; then
        systemctl enable "${BINARY_NAME}"
        info "Service enabled but not started (--no-start)."
        info "To start: sudo systemctl start ${BINARY_NAME}"
        SHOW_HINTS=false
    else
        systemctl enable --now "${BINARY_NAME}"
    fi
fi

# 9. Print post-install hints (only when fully activated)
if $SHOW_HINTS; then
    echo
    info "✓ ${BINARY_NAME} installed and started."
    echo
    if [[ "$OS_NAME" == "Darwin" ]]; then
        echo "  Status:    sudo launchctl print system/${LAUNCHD_LABEL}"
        echo "  Logs:      tail -f ${LOG_DIR}/${BINARY_NAME}.log"
        echo "  Restart:   sudo launchctl kickstart -k system/${LAUNCHD_LABEL}"
    else
        echo "  Status:    systemctl status ${BINARY_NAME}"
        echo "  Logs:      journalctl -u ${BINARY_NAME} -f"
        echo "  Restart:   systemctl restart ${BINARY_NAME}"
    fi
    echo "  Uninstall: sudo $(basename "${BASH_SOURCE[0]}") --uninstall"
fi
