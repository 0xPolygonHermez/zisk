#!/usr/bin/env bash
# install.sh — install zisk-worker as a systemd service on Linux.
#
# Usage:
#   sudo ./install.sh [OPTIONS]
#
# Options:
#   --env PATH            Load env vars from PATH (default: ./.env if present)
#   --binary PATH         Use a pre-built binary instead of building from source
#   --config PATH         Install an existing worker.toml instead of the sample
#   --proving-key PATH    Path to the proving key directory
#                         (default: /var/lib/zisk-worker/provingKey)
#   --mpi                 Force-enable MPI (default on Linux)
#   --no-mpi              Run the worker as a single process, no mpirun
#   --mpi-processes N     Manual override for -np
#   --mpi-numa-ppr N      Manual override for -map-by ppr:N:numa
#   --mpi-threads N       Manual override for RAYON_NUM_THREADS
#                         (--mpi-processes / --mpi-numa-ppr / --mpi-threads
#                          must all be specified together if any are given)
#   --no-start            Enable but do not start the service
#   --no-enable           Install unit file but do not enable or start
#   --uninstall           Stop, disable, and remove the service and binary
#
# Env-var equivalents (CLI flags win): ZISK_WORKER_BINARY, ZISK_WORKER_CONFIG,
# ZISK_WORKER_PROVING_KEY, ZISK_WORKER_MPI (true|false),
# ZISK_WORKER_MPI_PROCESSES, ZISK_WORKER_MPI_NUMA_PPR, ZISK_WORKER_MPI_THREADS.
#
# What this script does:
#   1. Verifies it's running on Linux
#   2. Creates the 'zisk-worker' system user
#   3. Builds or installs the binary to /usr/local/bin/zisk-worker
#   4. Installs config to /etc/zisk/worker.toml
#   5. Creates working directories (/var/lib/zisk-worker, /var/log/zisk)
#   6. Detects MPI parameters (unless --no-mpi)
#   7. Writes /etc/systemd/system/zisk-worker.service
#   8. Runs: systemctl daemon-reload && systemctl enable --now zisk-worker

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

BINARY_SRC="${ZISK_WORKER_BINARY:-}"
CONFIG_SRC="${ZISK_WORKER_CONFIG:-}"
PROVING_KEY="${ZISK_WORKER_PROVING_KEY:-$PROVING_KEY_DEFAULT}"
MPI_ENABLED="${ZISK_WORKER_MPI:-true}"
MPI_NP="${ZISK_WORKER_MPI_PROCESSES:-}"
MPI_PPR="${ZISK_WORKER_MPI_NUMA_PPR:-}"
MPI_THREADS="${ZISK_WORKER_MPI_THREADS:-}"
NO_START=false
NO_ENABLE=false
UNINSTALL=false

case "$MPI_ENABLED" in
    true|false) ;;
    *) die "ZISK_WORKER_MPI must be 'true' or 'false', got: ${MPI_ENABLED}" ;;
esac

while [[ $# -gt 0 ]]; do
    case "$1" in
        --env)            shift 2 ;;     # already consumed by load_env_file
        --binary)         BINARY_SRC="$2";    shift 2 ;;
        --config)         CONFIG_SRC="$2";    shift 2 ;;
        --proving-key)    PROVING_KEY="$2";   shift 2 ;;
        --mpi)            MPI_ENABLED=true;   shift ;;
        --no-mpi)         MPI_ENABLED=false;  shift ;;
        --mpi-processes)  MPI_NP="$2";        shift 2 ;;
        --mpi-numa-ppr)   MPI_PPR="$2";       shift 2 ;;
        --mpi-threads)    MPI_THREADS="$2";   shift 2 ;;
        --no-start)       NO_START=true;      shift ;;
        --no-enable)      NO_ENABLE=true;     shift ;;
        --uninstall)      UNINSTALL=true;     shift ;;
        *) die "Unknown option: $1" ;;
    esac
done

# ── uninstall ─────────────────────────────────────────────────────────────────

if $UNINSTALL; then
    need_root
    info "Uninstalling ${BINARY_NAME}..."
    systemctl stop    "${BINARY_NAME}" 2>/dev/null || true
    systemctl disable "${BINARY_NAME}" 2>/dev/null || true
    rm -f "${UNIT_FILE}" "${BINARY_DST}"
    systemctl daemon-reload
    info "Done. Config and data directories are left in place."
    info "Remove manually if no longer needed:"
    info "  sudo rm -rf ${CONFIG_DIR} ${WORK_DIR} ${LOG_DIR}"
    exit 0
fi

# ── install ───────────────────────────────────────────────────────────────────

need_root

# 1. Resolve MPI configuration
resolve_mpi_config "Install OpenMPI, load the module, or pass --no-mpi."

# 2. Build or use pre-built binary
build_or_use_binary "zisk-worker"

# 3. Create system group + user
if ! getent group "${SERVICE_GROUP}" &>/dev/null; then
    info "Creating system group '${SERVICE_GROUP}'..."
    groupadd --system "${SERVICE_GROUP}"
fi
if ! id "${SERVICE_USER}" &>/dev/null; then
    info "Creating system user '${SERVICE_USER}'..."
    useradd --system --gid "${SERVICE_GROUP}" --no-create-home --shell /usr/sbin/nologin "${SERVICE_USER}"
fi

# 4. Install binary
info "Installing binary to ${BINARY_DST}..."
install -m 755 "${BINARY_SRC}" "${BINARY_DST}"

# 5. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${SERVICE_GROUP}" \
    "${WORKSPACE_ROOT}/distributed/crates/worker/config/prod.toml"

# 6. Create working and log directories
mkdir -p "${WORK_DIR}" "${WORK_DIR}/inputs" "${WORK_DIR}/.zisk/cache" "${LOG_DIR}"
chown -R "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}" "${LOG_DIR}"

# 7. Build ExecStart line
if $MPI_ENABLED; then
    EXEC_START="ExecStart=${MPIRUN_BIN} --report-bindings --allow-run-as-root \\
    -np ${MPI_NP} \\
    -map-by ppr:${MPI_PPR}:numa \\
    --bind-to numa \\
    --rank-by slot \\
    -x RAYON_NUM_THREADS=${MPI_THREADS} \\
    ${BINARY_DST} --config ${CONFIG_DST} --proving-key ${PROVING_KEY}"
else
    EXEC_START="ExecStart=${BINARY_DST} --config ${CONFIG_DST} --proving-key ${PROVING_KEY}"
fi

# 8. Write systemd unit file
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
ReadOnlyPaths=${CONFIG_DIR} ${PROVING_KEY}

[Install]
WantedBy=multi-user.target
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
