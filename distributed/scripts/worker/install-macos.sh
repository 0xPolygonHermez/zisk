#!/usr/bin/env bash
# install-macos.sh — install zisk-worker as a launchd daemon on macOS.
#
# Usage:
#   sudo ./install-macos.sh [OPTIONS]
#
# Options:
#   --env PATH            Load env vars from PATH (default: ./.env if present)
#   --binary PATH         Use a pre-built binary instead of building from source
#   --config PATH         Install an existing worker.toml instead of the sample
#   --proving-key PATH    Path to the proving key directory
#                         (default: /var/lib/zisk-worker/provingKey)
#   --mpi                 Force-enable MPI (default OFF on macOS)
#   --no-mpi              Run the worker as a single process (default)
#   --mpi-processes N     Manual override for -np
#   --mpi-numa-ppr N      Manual override for -map-by ppr:N:numa
#   --mpi-threads N       Manual override for RAYON_NUM_THREADS
#   --uninstall           Stop, unload, and remove the service and binary
#
# Env-var equivalents (CLI flags win): ZISK_WORKER_BINARY, ZISK_WORKER_CONFIG,
# ZISK_WORKER_PROVING_KEY, ZISK_WORKER_MPI (true|false),
# ZISK_WORKER_MPI_PROCESSES, ZISK_WORKER_MPI_NUMA_PPR, ZISK_WORKER_MPI_THREADS.
#
# Notes:
#   macOS has no NUMA support and no CUDA on Apple Silicon. MPI on macOS is
#   only useful for single-host multi-process testing. Default is --no-mpi.
#
# What this script does:
#   1. Verifies it's running on macOS
#   2. Creates the 'zisk-worker' system user via dscl
#   3. Builds or installs the binary to /usr/local/bin/zisk-worker
#   4. Installs config to /etc/zisk/worker.toml
#   5. Creates working directories (/var/lib/zisk-worker, /var/log/zisk-worker)
#   6. Writes /Library/LaunchDaemons/com.zisk.worker.plist
#   7. Writes /etc/newsyslog.d/zisk-worker.conf for log rotation
#   8. Loads the service via launchctl

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_DIR="${SCRIPT_DIR}/../common"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

# shellcheck source=../common/lib.sh
source "${COMMON_DIR}/lib.sh"
# shellcheck source=./defaults.env
source "${SCRIPT_DIR}/defaults.env"

require_os "Darwin"

# ── load .env (if any), then argument parsing ─────────────────────────────────

load_env_file "$@"

BINARY_SRC="${ZISK_WORKER_BINARY:-}"
CONFIG_SRC="${ZISK_WORKER_CONFIG:-}"
PROVING_KEY="${ZISK_WORKER_PROVING_KEY:-$PROVING_KEY_DEFAULT}"
MPI_ENABLED="${ZISK_WORKER_MPI:-false}"   # macOS default: MPI off
MPI_NP="${ZISK_WORKER_MPI_PROCESSES:-}"
MPI_PPR="${ZISK_WORKER_MPI_NUMA_PPR:-}"
MPI_THREADS="${ZISK_WORKER_MPI_THREADS:-}"
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
        --uninstall)      UNINSTALL=true;     shift ;;
        *) die "Unknown option: $1" ;;
    esac
done

# ── uninstall ─────────────────────────────────────────────────────────────────

if $UNINSTALL; then
    need_root
    info "Uninstalling ${BINARY_NAME}..."
    launchctl unload "${LAUNCHD_PLIST}" 2>/dev/null || true
    rm -f "${LAUNCHD_PLIST}" "${BINARY_DST}" "${NEWSYSLOG_CONF}"
    info "Done. Config and data directories are left in place."
    info "Remove manually if no longer needed:"
    info "  sudo rm -rf ${CONFIG_DIR} ${WORK_DIR} ${LOG_DIR}"
    exit 0
fi

# ── install ───────────────────────────────────────────────────────────────────

need_root

# 1. Resolve MPI configuration
resolve_mpi_config "Install OpenMPI (brew install open-mpi), or pass --no-mpi."

# 2. Build or use pre-built binary
build_or_use_binary "zisk-worker"

# 3. Create system group + user via dscl
darwin_create_service_user "${SERVICE_USER}" "${SERVICE_GROUP}" "ZisK Worker" "${WORK_DIR}"

# 4. Install binary
info "Installing binary to ${BINARY_DST}..."
install -m 755 -o root -g wheel "${BINARY_SRC}" "${BINARY_DST}"

# 5. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${SERVICE_GROUP}" \
    "${WORKSPACE_ROOT}/distributed/crates/worker/config/prod.toml"

# 6. Create working and log directories. Pre-create ~/.zisk/cache for HOME-resolution.
mkdir -p "${WORK_DIR}" "${WORK_DIR}/inputs" "${WORK_DIR}/.zisk/cache" "${LOG_DIR}"
chown -R "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}" "${LOG_DIR}"

# 7. Build ProgramArguments array
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
}

# 8. Write launchd plist
info "Writing plist to ${LAUNCHD_PLIST}..."
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
PLIST

chown root:wheel "${LAUNCHD_PLIST}"
chmod 0644 "${LAUNCHD_PLIST}"

# 9. Write newsyslog rotation config
info "Writing newsyslog config to ${NEWSYSLOG_CONF}..."
cat > "${NEWSYSLOG_CONF}" <<NEWSYSLOG
# ${BINARY_NAME} log rotation — max ${LOG_MAX_SIZE_MB}MB per file, keep ${LOG_ROTATIONS} rotations, gzipped
${LOG_DIR}/${BINARY_NAME}.log  ${SERVICE_USER}:${SERVICE_GROUP}  640  ${LOG_ROTATIONS}  $(( LOG_MAX_SIZE_MB * 1024 ))  *  JG
NEWSYSLOG
chmod 0644 "${NEWSYSLOG_CONF}"

# 10. Load the service
info "Loading service via launchctl..."
launchctl unload "${LAUNCHD_PLIST}" 2>/dev/null || true
launchctl load -w "${LAUNCHD_PLIST}"

echo
info "✓ ${BINARY_NAME} installed and started."
echo
echo "  Status:    sudo launchctl print system/${LAUNCHD_LABEL}"
echo "  Logs:      tail -f ${LOG_DIR}/${BINARY_NAME}.log"
echo "  Restart:   sudo launchctl kickstart -k system/${LAUNCHD_LABEL}"
echo "  Uninstall: sudo $(basename "${BASH_SOURCE[0]}") --uninstall"
