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
#                          default: ${BUNDLE_DIR}/provingKey.
#   --with-snark           Download also the SNARK proving key into the bundle.
#                          Without this flag, only the STARK key is installed.
#   --proving-key-snark PATH  Path to the SNARK proving key directory (optional)
#   --coordinator-url URL  Distributed coordinator URL (overrides TOML)
#   --worker-id ID         Worker identifier (overrides TOML; auto-UUID if unset)
#   --compute-capacity N   Compute units to advertise (overrides TOML)
#   --emulator             Use prebuilt emulator (mutex with --asm)
#   --asm PATH             ASM file path (mutex with --emulator)
#   --gpu                  Install the GPU build and run worker with --gpu.
#   --cpu                  Install the CPU-only build and run worker without --gpu.
#                          Pass neither to let ziskup auto-detect CUDA on the
#                          host; the worker's runtime --gpu flag is then aligned
#                          to whichever variant ziskup picked.
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
#   --uninstall            Stop, disable, and remove the service
#   -y, --yes              Skip uninstall confirmation
#
# Notes:
#   On macOS, MPI defaults off — Apple Silicon has no NUMA, no CUDA. MPI on
#   macOS is only useful for single-host multi-process testing.
#
# Env-var equivalents (CLI flags win): ZISK_WORKER_BINARY, ZISK_WORKER_CONFIG,
# ZISK_WORKER_PROVING_KEY, ZISK_WORKER_PROVING_KEY_SNARK,
# ZISK_WORKER_COORDINATOR_URL, ZISK_WORKER_ID,
# ZISK_WORKER_COMPUTE_CAPACITY, ZISK_WORKER_EMULATOR (true|false), ZISK_WORKER_ASM,
# ZISK_WORKER_GPU (true|false; unset = auto-detect via ziskup), ZISK_WORKER_MPI (true|false),
# ZISK_WORKER_MPI_PROCESSES, ZISK_WORKER_MPI_NUMA_PPR, ZISK_WORKER_MPI_THREADS,
# ZISK_WORKER_WITH_SNARK (true|false), RUST_LOG.
#
# ── file layout (where install.sh puts things) ───────────────────────────────
# Runtime paths are resolved by common/src/paths.rs::ZiskPaths via ZISK_HOME
# (the shared bundle) and ZISK_CACHE_DIR (per-service writable cache). Both
# env vars are baked into the systemd unit / launchd plist below.
#
# Linux service mode:
#   Binary               /usr/local/bin/zisk-worker
#   Config               /etc/zisk/worker.toml
#   Systemd unit         /etc/systemd/system/zisk-worker.service
#   Logs                 journald (no on-disk log dir)
#   Service user         zisk-worker:zisk-worker (+ supplementary 'zisk')
#   State (writable)     /var/lib/zisk-worker/
#     ZISK_CACHE_DIR       /var/lib/zisk-worker/cache/   (ELF cache, ROM histograms)
#     inputs               /var/lib/zisk-worker/inputs/
#   Bundle (read-only)   /opt/zisk/                     ← ZISK_HOME (shared with coordinator)
#     binaries             /opt/zisk/bin/                (zisk-worker, libziskclib.a, ziskemu, …)
#     emulator-asm         /opt/zisk/zisk/emulator-asm/  (asm runners)
#     rust toolchains      /opt/zisk/toolchains/         (rustup-managed; guest compilation)
#     proving key          /opt/zisk/provingKey/         (default; --proving-key overrides)
#     SNARK key            /opt/zisk/provingKeySnark/    (only when --with-snark)
#     verify key           /opt/zisk/verifyKey/          (optional; populated by `ziskup --system --verifykey`)
#
# macOS service mode (bundle root differs — FHS doesn't apply on macOS):
#   Binary               /usr/local/bin/zisk-worker
#   Config               /etc/zisk/worker.toml
#   launchd plist        /Library/LaunchDaemons/com.zisk.worker.plist
#   Log file             /var/log/zisk-worker/zisk-worker.log  (rotated by newsyslog)
#   newsyslog config     /etc/newsyslog.d/zisk-worker.conf
#   State (writable)     /usr/local/var/zisk-worker/    (mac /var/lib trips SIP)
#     ZISK_CACHE_DIR       /usr/local/var/zisk-worker/cache/
#     inputs               /usr/local/var/zisk-worker/inputs/
#   Bundle (read-only)   /Library/Application Support/ZisK/  ← ZISK_HOME

# ── self-bootstrap (curl-pipe-able install) ──────────────────────────────────
# When this script runs without its sibling files (curl | bash, or copied
# without lib.sh/defaults.env/mpi_params.sh/ziskup nearby), download the
# deploy tree from GitHub and re-exec from the temp copy.
#
# Usage from a fresh server (no clone needed):
#
#   curl -fsL https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/distributed/deploy/scripts/worker/install.sh \
#       | sudo bash -s -- --gpu --config /etc/zisk/worker.toml --coordinator-url <URL>
#
#   # Pin a non-default branch (e.g., a PR under review):
#   curl -fsL .../install.sh | sudo ZISK_DEPLOY_BRANCH=feature/foo bash -s -- ...
_self="${BASH_SOURCE[0]:-}"
SELF_DIR=""
if [[ -n "$_self" && -f "$_self" ]]; then
    SELF_DIR="$(cd "$(dirname "$_self")" 2>/dev/null && pwd)" || SELF_DIR=""
fi
unset _self
if [[ -z "${SELF_DIR}" \
   || ! -f "${SELF_DIR}/../common/lib.sh" \
   || ! -f "${SELF_DIR}/defaults.env" ]]; then
    echo "[bootstrap] no sibling deploy scripts found; fetching from GitHub..."
    # Prefer /var/tmp (FHS non-volatile temp; almost always exec-allowed) over
    # /tmp (often mounted noexec on hardened or container environments).
    BOOTSTRAP_TMP=$(mktemp -d /var/tmp/zisk-deploy.XXXXXX 2>/dev/null) \
                || BOOTSTRAP_TMP=$(mktemp -d /tmp/zisk-deploy.XXXXXX)
    # Override with ZISK_DEPLOY_BRANCH=<branch|tag> to pin a non-default ref
    # (a feature branch under review, or a release tag for reproducible installs).
    BRANCH="${ZISK_DEPLOY_BRANCH:-main}"
    BASE="https://raw.githubusercontent.com/0xPolygonHermez/zisk/${BRANCH}"
    mkdir -p \
        "${BOOTSTRAP_TMP}/distributed/deploy/scripts/common" \
        "${BOOTSTRAP_TMP}/distributed/deploy/scripts/worker" \
        "${BOOTSTRAP_TMP}/distributed/crates/worker/config" \
        "${BOOTSTRAP_TMP}/ziskup"
    for f in \
        distributed/deploy/scripts/common/lib.sh \
        distributed/deploy/scripts/common/mpi_params.sh \
        distributed/deploy/scripts/worker/install.sh \
        distributed/deploy/scripts/worker/defaults.env \
        distributed/crates/worker/config/prod.toml \
        ziskup/ziskup ; do
        if ! curl -fsL --retry 3 --max-time 30 "${BASE}/${f}" -o "${BOOTSTRAP_TMP}/${f}"; then
            echo "[bootstrap] failed to download ${f} from ${BRANCH}" >&2
            rm -rf "${BOOTSTRAP_TMP}"
            exit 1
        fi
    done
    chmod +x "${BOOTSTRAP_TMP}/distributed/deploy/scripts/common/mpi_params.sh" \
             "${BOOTSTRAP_TMP}/distributed/deploy/scripts/worker/install.sh" \
             "${BOOTSTRAP_TMP}/ziskup/ziskup"
    # Clean up the bootstrap dir on exit (success, failure, or interrupt).
    trap 'rm -rf "${BOOTSTRAP_TMP}"' EXIT
    bash "${BOOTSTRAP_TMP}/distributed/deploy/scripts/worker/install.sh" "$@"
    exit $?
fi

set -euo pipefail

SCRIPT_DIR="${SELF_DIR}"
COMMON_DIR="${SCRIPT_DIR}/../common"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"
# Bootstrap-safe root for deploy assets (works in repo clone and temp fetch).
DEPLOY_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

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
# GPU is tristate: empty = let ziskup auto-detect CUDA, "true"/"false" = explicit.
# Resolved to true/false after ziskup runs (either from operator intent or by
# reading .zisk-bundle's `variant` field).
GPU="${ZISK_WORKER_GPU:-}"
LOG_LEVEL="${RUST_LOG:-}"
MPI_ENABLED="${ZISK_WORKER_MPI:-$MPI_DEFAULT}"
MPI_NP="${ZISK_WORKER_MPI_PROCESSES:-}"
MPI_PPR="${ZISK_WORKER_MPI_NUMA_PPR:-}"
MPI_THREADS="${ZISK_WORKER_MPI_THREADS:-}"
WITH_SNARK="${ZISK_WORKER_WITH_SNARK:-false}"
NO_START=false
NO_ENABLE=false
UNINSTALL=false
ASSUME_YES=false

case "$MPI_ENABLED" in true|false) ;; *) die "ZISK_WORKER_MPI must be 'true' or 'false', got: ${MPI_ENABLED}" ;; esac
case "$EMULATOR"    in true|false) ;; *) die "ZISK_WORKER_EMULATOR must be 'true' or 'false', got: ${EMULATOR}" ;; esac
case "$GPU"         in true|false|"") ;; *) die "ZISK_WORKER_GPU must be 'true', 'false', or unset, got: ${GPU}" ;; esac
case "$WITH_SNARK"  in true|false) ;; *) die "ZISK_WORKER_WITH_SNARK must be 'true' or 'false', got: ${WITH_SNARK}" ;; esac

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
        --gpu)               GPU=true;  shift ;;
        --cpu)               GPU=false; shift ;;
        --log-level)         LOG_LEVEL="$2";        shift 2 ;;
        --mpi)               MPI_ENABLED=true;      shift ;;
        --no-mpi)            MPI_ENABLED=false;     shift ;;
        --mpi-processes)     MPI_NP="$2";           shift 2 ;;
        --mpi-numa-ppr)      MPI_PPR="$2";          shift 2 ;;
        --mpi-threads)       MPI_THREADS="$2";      shift 2 ;;
        --with-snark)        WITH_SNARK=true;       shift ;;
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
# Downloads the STARK proving key into ${BUNDLE_DIR}/provingKey (the default
# --proving-key location); pass --proving-key <path> to point the worker at a
# pre-staged key elsewhere (the bundle copy is still downloaded).
# --with-snark additionally downloads the SNARK proving key (only valid with --system).
ZISKUP_BIN="$(resolve_ziskup_bin)"
ZISKUP_ARGS=(--system --prefix "${BUNDLE_DIR}" --owner zisk:zisk --yes)
# Pin ziskup's binary selection only when the operator was explicit. Without
# a flag, let ziskup auto-detect CUDA and read its choice back from
# .zisk-bundle below to align the worker's runtime --gpu flag.
case "$GPU" in
    true)  ZISKUP_ARGS+=(--gpu) ;;
    false) ZISKUP_ARGS+=(--cpu) ;;
esac
$WITH_SNARK && ZISKUP_ARGS+=(--with-snark)
info "Populating ${BUNDLE_DIR} via ${ZISKUP_BIN} ${ZISKUP_ARGS[*]}..."
"${ZISKUP_BIN}" "${ZISKUP_ARGS[@]}"

# Resolve the variant from .zisk-bundle when the operator didn't pin it
# explicitly. The bundle metadata file is a documented contract — see the
# schema header in ziskup/ziskup.
if [[ -z "$GPU" ]]; then
    variant=$(bundle_meta_get variant)
    case "$variant" in
        gpu) GPU=true ;;
        cpu) GPU=false ;;
        *) die "Could not determine variant from ${BUNDLE_DIR}/.zisk-bundle (got '${variant:-empty}'). Pass --gpu or --cpu explicitly." ;;
    esac
    info "Worker variant resolved from ziskup: ${variant}"
fi

# Default --proving-key-snark to the bundle's location when --with-snark was
# passed (operator can still override with --proving-key-snark <path>).
$WITH_SNARK && : "${PROVING_KEY_SNARK:=${BUNDLE_DIR}/provingKeySnark}"

# 3. Resolve the zisk-worker binary (from --binary if given, else from the
# bundle ziskup just populated). NO local cargo build.
resolve_service_binary

# 4. Create system group + user (with 'zisk' as supplementary group so it can
# read the bundle). Home is /var/empty (matches coordinator + sshd convention);
# pointing it at WORK_DIR triggers macOS dscl-managed-home ACLs that block
# rm -rf during uninstall.
create_service_user "${SERVICE_USER}" "${SERVICE_GROUP}" "ZisK Worker" "/var/empty"
add_user_to_group "${SERVICE_USER}" zisk

# 5. Install binary
install_binary "${BINARY_SRC}" "${BINARY_DST}"

# 6. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${SERVICE_GROUP}" \
    "${DEPLOY_ROOT}/crates/worker/config/prod.toml"

# 7. Create per-service working dirs (mutable runtime state only). LOG_DIR is
# only created on macOS — Linux pipes logs to journald, no on-disk log dir
# needed (and creating it leaves a dangling empty dir that --uninstall would
# prompt about).
mkdir -p "${WORK_DIR}" "${WORK_DIR}/inputs" "${WORK_DIR}/cache"
chown -R "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}"
if [[ "$OS_NAME" == "Darwin" ]]; then
    mkdir -p "${LOG_DIR}"
    chown -R "${SERVICE_USER}:${SERVICE_GROUP}" "${LOG_DIR}"
fi

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
            # See the systemd ExecStart above for why ZISK_HOME, ZISK_CACHE_DIR,
            # and HOME are forwarded to ranks explicitly via `-x VAR` rather
            # than relying on OpenMPI's default env propagation.
            printf '        <string>-x</string>\n'
            printf '        <string>ZISK_HOME</string>\n'
            printf '        <string>-x</string>\n'
            printf '        <string>ZISK_CACHE_DIR</string>\n'
            printf '        <string>-x</string>\n'
            printf '        <string>HOME</string>\n'
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
<!-- ${BINARY_NAME}:CONFIG_FILE=${CONFIG_DST} -->
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
        # `-x VAR` (no value) propagates the unit's Environment= entries to
        # MPI ranks. Without these explicit entries, ranks inherit OpenMPI's
        # default-env-propagation behaviour, which various OMPI builds and
        # tight-integration launchers (PBS, SLURM, etc.) filter differently
        # — so the worker's ZiskPaths::from_env() falls back to $HOME/.zisk
        # in the rank's process. Forward HOME alongside as a safety net for
        # the same fallback path.
        EXEC_START="ExecStart=${MPIRUN_BIN} --report-bindings --allow-run-as-root \\
    -np ${MPI_NP} \\
    -map-by ppr:${MPI_PPR}:numa \\
    --bind-to numa \\
    --rank-by slot \\
    -x RAYON_NUM_THREADS=${MPI_THREADS} \\
    -x ZISK_HOME \\
    -x ZISK_CACHE_DIR \\
    -x HOME \\
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

    # ReadWritePaths punch holes in ReadOnlyPaths for key dirs. The worker
    # writes generated artefacts (const tree files, intermediates) into
    # these at runtime; ziskup's apply_ownership_and_perms makes them 0770
    # so the supplementary 'zisk' group has filesystem write, and these
    # entries lift the systemd namespace-level RO mount over them. Use the
    # `-` prefix so missing dirs (e.g. verifyKey when not installed) are
    # silently ignored instead of failing service start.
    READ_WRITE_PATHS="${WORK_DIR}"
    READ_WRITE_PATHS+=" -${BUNDLE_DIR}/provingKey"
    READ_WRITE_PATHS+=" -${BUNDLE_DIR}/provingKeySnark"
    READ_WRITE_PATHS+=" -${BUNDLE_DIR}/verifyKey"
    # Worker setup invokes `make` in zisk/emulator-asm with current_dir there;
    # the Makefile mkdir's build/ inside the source dir.
    READ_WRITE_PATHS+=" -${BUNDLE_DIR}/zisk/emulator-asm"

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
ReadWritePaths=${READ_WRITE_PATHS}
ReadOnlyPaths=${READ_ONLY_PATHS}

[Install]
WantedBy=multi-user.target

# Install metadata (read by --uninstall; do not edit)
# ${BINARY_NAME}:DATA_DIR=${WORK_DIR}
# ${BINARY_NAME}:LOG_DIR=${LOG_DIR}
# ${BINARY_NAME}:CONFIG_DIR=${CONFIG_DIR}
# ${BINARY_NAME}:CONFIG_FILE=${CONFIG_DST}
# ${BINARY_NAME}:SVC_USER=${SERVICE_USER}
# ${BINARY_NAME}:SVC_GROUP=${SERVICE_GROUP}
EOF
fi

# 10. Activate service and (if started) print management hints
activate_service
if $SHOW_HINTS; then
    print_post_install_hints "${BASH_SOURCE[0]}"
fi
