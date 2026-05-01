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
#   --proving-key-snark PATH  Path to the SNARK proving key directory (optional)
#   --coordinator-url URL  Distributed coordinator URL (overrides TOML)
#   --worker-id ID         Worker identifier (overrides TOML; auto-UUID if unset)
#   --compute-capacity N   Compute units to advertise (overrides TOML)
#   --emulator             Use prebuilt emulator (mutex with --asm)
#   --asm PATH             ASM file path (mutex with --emulator)
#   --gpu                  Run with GPU (only meaningful in non-cpu-only builds)
#   --log-level LEVEL      trace | debug | info | warn | error (optional; RUST_LOG)
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
# ZISK_WORKER_PROVING_KEY, ZISK_WORKER_PROVING_KEY_SNARK,
# ZISK_WORKER_COORDINATOR_URL, ZISK_WORKER_ID,
# ZISK_WORKER_COMPUTE_CAPACITY, ZISK_WORKER_EMULATOR (true|false), ZISK_WORKER_ASM,
# ZISK_WORKER_GPU (true|false), ZISK_WORKER_MPI (true|false),
# ZISK_WORKER_MPI_PROCESSES, ZISK_WORKER_MPI_NUMA_PPR, ZISK_WORKER_MPI_THREADS,
# RUST_LOG.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_DIR="${SCRIPT_DIR}/../common"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"

# shellcheck source=../common/lib.sh
source "${COMMON_DIR}/lib.sh"
# shellcheck source=./defaults.env
source "${SCRIPT_DIR}/defaults.env"

print_banner worker

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
PROVING_KEY_SNARK="${ZISK_WORKER_PROVING_KEY_SNARK:-}"
COORDINATOR_URL="${ZISK_WORKER_COORDINATOR_URL:-}"
WORKER_ID="${ZISK_WORKER_ID:-}"
COMPUTE_CAPACITY="${ZISK_WORKER_COMPUTE_CAPACITY:-}"
EMULATOR="${ZISK_WORKER_EMULATOR:-false}"
ASM_PATH="${ZISK_WORKER_ASM:-}"
GPU="${ZISK_WORKER_GPU:-false}"
LOG_LEVEL="${RUST_LOG:-}"
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
        --proving-key-snark) PROVING_KEY_SNARK="$2"; shift 2 ;;
        --coordinator-url)   COORDINATOR_URL="$2";  shift 2 ;;
        --worker-id)         WORKER_ID="$2";        shift 2 ;;
        --compute-capacity)  COMPUTE_CAPACITY="$2"; shift 2 ;;
        --emulator)          EMULATOR=true;         shift ;;
        --asm)               ASM_PATH="$2";         shift 2 ;;
        --gpu)               GPU=true;              shift ;;
        --log-level)         LOG_LEVEL="$2";        shift 2 ;;
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
    uninstall_service
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

# 2. Populate the shared ZisK bundle at ${BUNDLE_DIR} via ziskup. ziskup --system
# creates the 'zisk' system user/group, downloads the release tarball, extracts
# to ${BUNDLE_DIR}, applies 0750 perms. Idempotent — safe to re-run.
# --nokey skips the inline proving-key download (operator passes --proving-key).
ZISKUP_BIN="$(resolve_ziskup_bin)"
info "Populating ${BUNDLE_DIR} via ${ZISKUP_BIN} --system..."
"${ZISKUP_BIN}" --system --prefix "${BUNDLE_DIR}" --owner zisk:zisk --yes --nokey

# 3. Resolve the zisk-worker binary (from --binary if given, else from the
# bundle ziskup just populated). NO local cargo build.
resolve_service_binary "zisk-worker"

# 4. Create system group + user (with 'zisk' as supplementary group so it can
# read the bundle).
create_service_user "${SERVICE_USER}" "${SERVICE_GROUP}" "ZisK Worker" "${WORK_DIR}"
add_user_to_group "${SERVICE_USER}" zisk

# 5. Install binary
install_binary "${BINARY_SRC}" "${BINARY_DST}"

# 6. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${SERVICE_GROUP}" \
    "${WORKSPACE_ROOT}/distributed/crates/worker/config/prod.toml"

# 7. Create per-service working dirs (mutable runtime state only) and log dir.
mkdir -p "${WORK_DIR}" "${WORK_DIR}/inputs" "${WORK_DIR}/cache" "${LOG_DIR}"
chown -R "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}" "${LOG_DIR}"

# 8. Write service unit
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
        if [[ -n "$PROVING_KEY_SNARK" ]]; then
            printf '        <string>--proving-key-snark</string>\n'
            printf '        <string>%s</string>\n' "${PROVING_KEY_SNARK}"
        fi
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

    <key>EnvironmentVariables</key>
    <dict>
        <key>ZISK_HOME</key>
        <string>${BUNDLE_DIR}</string>
        <key>ZISK_CACHE_DIR</key>
        <string>${WORK_DIR}/cache</string>
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

<!-- Install metadata (read at uninstall time; do not edit) -->
<!-- ${BINARY_NAME}:DATA_DIR=${WORK_DIR} -->
<!-- ${BINARY_NAME}:LOG_DIR=${LOG_DIR} -->
<!-- ${BINARY_NAME}:CONFIG_DIR=${CONFIG_DIR} -->
<!-- ${BINARY_NAME}:SVC_USER=${SERVICE_USER} -->
<!-- ${BINARY_NAME}:SVC_GROUP=${SERVICE_GROUP} -->
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
else
    # Build worker args (everything after the binary path); appended to mpirun
    # wrapper or to bare ExecStart depending on MPI_ENABLED.
    WORKER_ARGS="--config ${CONFIG_DST} --proving-key ${PROVING_KEY}"
    [[ -n "$PROVING_KEY_SNARK" ]] && WORKER_ARGS+=" --proving-key-snark ${PROVING_KEY_SNARK}"
    [[ -n "$COORDINATOR_URL" ]]  && WORKER_ARGS+=" --coordinator-url ${COORDINATOR_URL}"
    [[ -n "$WORKER_ID" ]]        && WORKER_ARGS+=" --worker-id ${WORKER_ID}"
    [[ -n "$COMPUTE_CAPACITY" ]] && WORKER_ARGS+=" --compute-capacity ${COMPUTE_CAPACITY}"
    $EMULATOR                    && WORKER_ARGS+=" --emulator"
    [[ -n "$ASM_PATH" ]]         && WORKER_ARGS+=" --asm ${ASM_PATH}"
    $GPU                         && WORKER_ARGS+=" --gpu"
    [[ -n "$LOG_LEVEL" ]]        && WORKER_ARGS+=" --log-level ${LOG_LEVEL}"

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

    # ReadOnlyPaths: bundle is the toolchain payload + proving key; optional
    # ASM and SNARK key are appended only when explicitly set.
    READ_ONLY_PATHS="${CONFIG_DIR} ${BUNDLE_DIR}"
    [[ -n "$ASM_PATH" ]]          && READ_ONLY_PATHS+=" ${ASM_PATH}"
    [[ -n "$PROVING_KEY_SNARK" && "$PROVING_KEY_SNARK" != "${BUNDLE_DIR}"/* ]] \
        && READ_ONLY_PATHS+=" ${PROVING_KEY_SNARK}"

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
SupplementaryGroups=zisk
WorkingDirectory=${WORK_DIR}

# Resolver paths (see common/src/paths.rs::ZiskPaths).
Environment=ZISK_HOME=${BUNDLE_DIR}
Environment=ZISK_CACHE_DIR=${WORK_DIR}/cache

${EXEC_START}
Restart=on-failure
RestartSec=5

# Resource limits — large mmap'd ROM regions, deep MPI stacks, many open
# shm/data files. Use unlimited so the worker is constrained by physical
# resources, not by default systemd hardening.
LimitMEMLOCK=infinity
# LimitSTACK=infinity
# LimitNOFILE=infinity
# LimitNPROC=infinity
# LimitAS=infinity
# LimitDATA=infinity
# LimitCORE=infinity

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

# 10. Activate service and (if started) print management hints
activate_service
if $SHOW_HINTS; then
    print_post_install_hints "${BASH_SOURCE[0]}"
fi
