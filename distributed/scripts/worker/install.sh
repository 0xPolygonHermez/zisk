#!/usr/bin/env bash
# install.sh — install zisk-worker as a systemd service (Linux) or launchd
#              daemon (macOS).
#
# Usage:
#   sudo ./install.sh [OPTIONS]
#
# Options:
#   --env PATH             Load env vars from PATH (default: ./.env if present)
#   --binary PATH          Use a pre-built binary instead of building from source
#   --config PATH          Install an existing worker.toml instead of the sample
#   --proving-key PATH     Path to the proving key directory
#                          (default: /var/lib/zisk-worker/provingKey)
#   --coordinator-url URL  Distributed coordinator URL (overrides TOML)
#   --worker-id ID         Worker identifier (overrides TOML; auto-UUID if unset)
#   --compute-capacity N   Compute units to advertise (overrides TOML)
#   --emulator             Use prebuilt emulator (mutex with --asm)
#   --asm PATH             ASM file path (mutex with --emulator)
#   --gpu                  Run with GPU (only meaningful in non-cpu-only builds)
#   --mpi                  Force-enable MPI (default true on Linux, false on macOS)
#   --no-mpi               Run the worker as a single process, no mpirun
#   --mpi-processes N      Manual override for -np
#   --mpi-numa-ppr N       Manual override for -map-by ppr:N:numa
#   --mpi-threads N        Manual override for RAYON_NUM_THREADS
#                          (--mpi-processes / --mpi-numa-ppr / --mpi-threads
#                           must all be specified together if any are given)
#   --no-start             Linux: enable but don't start.
#                          macOS: write plist but don't load (same as --no-enable).
#   --no-enable            Linux: install unit but don't enable or start.
#                          macOS: write plist but don't load.
#   --uninstall            Stop, disable, and remove the service (prompts for cleanup)
#   -y, --yes              Skip every uninstall prompt (assume yes)
#
# Notes:
#   On macOS, MPI defaults off — Apple Silicon has no NUMA, no CUDA. MPI on
#   macOS is only useful for single-host multi-process testing.
#
# Env-var equivalents (CLI flags win): ZISK_WORKER_BINARY, ZISK_WORKER_CONFIG,
# ZISK_WORKER_PROVING_KEY, ZISK_WORKER_COORDINATOR_URL, ZISK_WORKER_ID,
# ZISK_WORKER_COMPUTE_CAPACITY, ZISK_WORKER_EMULATOR (true|false), ZISK_WORKER_ASM,
# ZISK_WORKER_GPU (true|false), ZISK_WORKER_MPI (true|false),
# ZISK_WORKER_MPI_PROCESSES, ZISK_WORKER_MPI_NUMA_PPR, ZISK_WORKER_MPI_THREADS.

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

# OS-aware MPI default — Linux on by default, Darwin off.
if [[ "$OS_NAME" == "Darwin" ]]; then
    MPI_DEFAULT=false
else
    MPI_DEFAULT=true
fi

# ── load .env (if any), then argument parsing ─────────────────────────────────

load_env_file "$@"

BINARY_SRC="${ZISK_WORKER_BINARY:-}"
CONFIG_SRC="${ZISK_WORKER_CONFIG:-}"
PROVING_KEY="${ZISK_WORKER_PROVING_KEY:-$PROVING_KEY_DEFAULT}"
COORDINATOR_URL="${ZISK_WORKER_COORDINATOR_URL:-}"
WORKER_ID="${ZISK_WORKER_ID:-}"
COMPUTE_CAPACITY="${ZISK_WORKER_COMPUTE_CAPACITY:-}"
EMULATOR="${ZISK_WORKER_EMULATOR:-false}"
ASM_PATH="${ZISK_WORKER_ASM:-}"
GPU="${ZISK_WORKER_GPU:-false}"
MPI_ENABLED="${ZISK_WORKER_MPI:-$MPI_DEFAULT}"
MPI_NP="${ZISK_WORKER_MPI_PROCESSES:-}"
MPI_PPR="${ZISK_WORKER_MPI_NUMA_PPR:-}"
MPI_THREADS="${ZISK_WORKER_MPI_THREADS:-}"
NO_START=false
NO_ENABLE=false
UNINSTALL=false
ASSUME_YES=false

case "$MPI_ENABLED" in true|false) ;; *) die "ZISK_WORKER_MPI must be 'true' or 'false', got: ${MPI_ENABLED}" ;; esac
case "$EMULATOR"    in true|false) ;; *) die "ZISK_WORKER_EMULATOR must be 'true' or 'false', got: ${EMULATOR}" ;; esac
case "$GPU"         in true|false) ;; *) die "ZISK_WORKER_GPU must be 'true' or 'false', got: ${GPU}" ;; esac

while [[ $# -gt 0 ]]; do
    case "$1" in
        --env)               shift 2 ;;     # already consumed by load_env_file
        --binary)            BINARY_SRC="$2";       shift 2 ;;
        --config)            CONFIG_SRC="$2";       shift 2 ;;
        --proving-key)       PROVING_KEY="$2";      shift 2 ;;
        --coordinator-url)   COORDINATOR_URL="$2";  shift 2 ;;
        --worker-id)         WORKER_ID="$2";        shift 2 ;;
        --compute-capacity)  COMPUTE_CAPACITY="$2"; shift 2 ;;
        --emulator)          EMULATOR=true;         shift ;;
        --asm)               ASM_PATH="$2";         shift 2 ;;
        --gpu)               GPU=true;              shift ;;
        --mpi)               MPI_ENABLED=true;      shift ;;
        --no-mpi)            MPI_ENABLED=false;     shift ;;
        --mpi-processes)     MPI_NP="$2";           shift 2 ;;
        --mpi-numa-ppr)      MPI_PPR="$2";          shift 2 ;;
        --mpi-threads)       MPI_THREADS="$2";      shift 2 ;;
        --no-start)          NO_START=true;         shift ;;
        --no-enable)         NO_ENABLE=true;        shift ;;
        --uninstall)         UNINSTALL=true;        shift ;;
        -y|--yes)            ASSUME_YES=true;       shift ;;
        *) die "Unknown option: $1" ;;
    esac
done

# --emulator and --asm are mutually exclusive (binary enforces it; we fail earlier)
if $EMULATOR && [[ -n "$ASM_PATH" ]]; then
    die "--emulator and --asm are mutually exclusive."
fi

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

# 1. Resolve MPI configuration
if [[ "$OS_NAME" == "Darwin" ]]; then
    resolve_mpi_config "Install OpenMPI (brew install open-mpi), or pass --no-mpi."
else
    resolve_mpi_config "Install OpenMPI, load the module, or pass --no-mpi."
fi

# 2. Build or use pre-built binary
build_or_use_binary "zisk-worker"

# 3. Create system group + user
create_service_user "${SERVICE_USER}" "${SERVICE_GROUP}" "ZisK Worker" "${WORK_DIR}"

# 4. Install binary
install_binary "${BINARY_SRC}" "${BINARY_DST}"

# 5. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${SERVICE_GROUP}" \
    "${WORKSPACE_ROOT}/distributed/crates/worker/config/prod.toml"

# 6. Create working and log directories. Pre-create ~/.zisk/cache for HOME-resolution.
mkdir -p "${WORK_DIR}" "${WORK_DIR}/inputs" "${WORK_DIR}/.zisk/cache" "${LOG_DIR}"
chown -R "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}" "${LOG_DIR}"

# 7. Write service unit
if [[ "$OS_NAME" == "Darwin" ]]; then
    info "Writing plist to ${LAUNCHD_PLIST}..."

    build_program_args() {
        if $MPI_ENABLED; then
            printf '        <string>%s</string>\n' "${MPIRUN_BIN}"
            printf '        <string>--report-bindings</string>\n'
            printf '        <string>--allow-run-as-root</string>\n'
            printf '        <string>-np</string>\n'
            printf '        <string>%s</string>\n' "${MPI_NP}"
            printf '        <string>-map-by</string>\n'
            printf '        <string>ppr:%s:numa</string>\n' "${MPI_PPR}"
            printf '        <string>--bind-to</string>\n'
            printf '        <string>numa</string>\n'
            printf '        <string>--rank-by</string>\n'
            printf '        <string>slot</string>\n'
            printf '        <string>-x</string>\n'
            printf '        <string>RAYON_NUM_THREADS=%s</string>\n' "${MPI_THREADS}"
        fi
        printf '        <string>%s</string>\n' "${BINARY_DST}"
        printf '        <string>--config</string>\n'
        printf '        <string>%s</string>\n' "${CONFIG_DST}"
        printf '        <string>--proving-key</string>\n'
        printf '        <string>%s</string>\n' "${PROVING_KEY}"
        if [[ -n "$COORDINATOR_URL" ]]; then
            printf '        <string>--coordinator-url</string>\n'
            printf '        <string>%s</string>\n' "${COORDINATOR_URL}"
        fi
        if [[ -n "$WORKER_ID" ]]; then
            printf '        <string>--worker-id</string>\n'
            printf '        <string>%s</string>\n' "${WORKER_ID}"
        fi
        if [[ -n "$COMPUTE_CAPACITY" ]]; then
            printf '        <string>--compute-capacity</string>\n'
            printf '        <string>%s</string>\n' "${COMPUTE_CAPACITY}"
        fi
        if $EMULATOR; then
            printf '        <string>--emulator</string>\n'
        fi
        if [[ -n "$ASM_PATH" ]]; then
            printf '        <string>--asm</string>\n'
            printf '        <string>%s</string>\n' "${ASM_PATH}"
        fi
        if $GPU; then
            printf '        <string>--gpu</string>\n'
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

    <key>EnvironmentVariables</key>
    <dict>
        <key>HOME</key>
        <string>${WORK_DIR}</string>
    </dict>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>${LOG_DIR}/${BINARY_NAME}.log</string>

    <key>StandardErrorPath</key>
    <string>${LOG_DIR}/${BINARY_NAME}.log</string>

    <key>ProcessType</key>
    <string>Interactive</string>

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

    # 8. Write newsyslog rotation config
    info "Writing newsyslog config to ${NEWSYSLOG_CONF}..."
    cat > "${NEWSYSLOG_CONF}" <<NEWSYSLOG
# ${BINARY_NAME} log rotation — max ${LOG_MAX_SIZE_MB}MB per file, keep ${LOG_ROTATIONS} rotations, gzipped
${LOG_DIR}/${BINARY_NAME}.log  ${SERVICE_USER}:${SERVICE_GROUP}  640  ${LOG_ROTATIONS}  $(( LOG_MAX_SIZE_MB * 1024 ))  *  JG
NEWSYSLOG
    chmod 0644 "${NEWSYSLOG_CONF}"
else
    # Build worker args (everything after the binary path); appended to mpirun
    # wrapper or to bare ExecStart depending on MPI_ENABLED.
    WORKER_ARGS="--config ${CONFIG_DST} --proving-key ${PROVING_KEY}"
    [[ -n "$COORDINATOR_URL" ]]  && WORKER_ARGS+=" --coordinator-url ${COORDINATOR_URL}"
    [[ -n "$WORKER_ID" ]]        && WORKER_ARGS+=" --worker-id ${WORKER_ID}"
    [[ -n "$COMPUTE_CAPACITY" ]] && WORKER_ARGS+=" --compute-capacity ${COMPUTE_CAPACITY}"
    $EMULATOR                    && WORKER_ARGS+=" --emulator"
    [[ -n "$ASM_PATH" ]]         && WORKER_ARGS+=" --asm ${ASM_PATH}"
    $GPU                         && WORKER_ARGS+=" --gpu"

    if $MPI_ENABLED; then
        EXEC_START="ExecStart=${MPIRUN_BIN} --report-bindings --allow-run-as-root \\
    -np ${MPI_NP} \\
    -map-by ppr:${MPI_PPR}:numa \\
    --bind-to numa \\
    --rank-by slot \\
    -x RAYON_NUM_THREADS=${MPI_THREADS} \\
    ${BINARY_DST} ${WORKER_ARGS}"
    else
        EXEC_START="ExecStart=${BINARY_DST} ${WORKER_ARGS}"
    fi

    # ReadOnlyPaths: include ASM_PATH so the sandbox can read it
    READ_ONLY_PATHS="${CONFIG_DIR} ${PROVING_KEY}"
    [[ -n "$ASM_PATH" ]] && READ_ONLY_PATHS+=" ${ASM_PATH}"

    info "Writing unit file to ${UNIT_FILE}..."
    cat > "${UNIT_FILE}" <<EOF
[Unit]
Description=ZisK Worker — distributed proof generation worker
Documentation=https://github.com/0xPolygonHermez/zisk
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${SERVICE_USER}
Group=${SERVICE_GROUP}
WorkingDirectory=${WORK_DIR}

# HOME is needed so the worker resolves ~/.zisk/cache correctly.
Environment=HOME=${WORK_DIR}

${EXEC_START}
Restart=on-failure
RestartSec=5
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
ReadWritePaths=${WORK_DIR} ${LOG_DIR}
ReadOnlyPaths=${READ_ONLY_PATHS}

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

# 9. Activate service (or skip)
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

# 10. Print post-install hints (only when fully activated)
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
