#!/usr/bin/env bash
# Shared bash helpers for ZisK install scripts.
# Source this file: source "${SCRIPT_DIR}/../common/lib.sh"
#
# ── Environment-variable convention ──────────────────────────────────────────
# Install scripts read the following env vars (typically populated from a .env
# file via load_env_file). CLI flags always override env vars; env vars
# override defaults.env. Only configuration values are exposed; operational
# toggles such as --uninstall, --no-start and --no-enable remain CLI-only.
#
#   Coordinator
#     ZISK_COORDINATOR_BINARY        pre-built binary path  (--binary)
#     ZISK_COORDINATOR_CONFIG        TOML config path       (--config)
#     ZISK_COORDINATOR_PORT          listening port         (--port)
#
#   Worker
#     ZISK_WORKER_BINARY             pre-built binary path  (--binary)
#     ZISK_WORKER_CONFIG             TOML config path       (--config)
#     ZISK_WORKER_PROVING_KEY        proving key directory  (--proving-key)
#     ZISK_WORKER_MPI                true|false             (--mpi / --no-mpi)
#     ZISK_WORKER_MPI_PROCESSES      -np override           (--mpi-processes)
#     ZISK_WORKER_MPI_NUMA_PPR       ppr:N:numa override    (--mpi-numa-ppr)
#     ZISK_WORKER_MPI_THREADS        RAYON_NUM_THREADS      (--mpi-threads)

info() { echo "[INFO]  $*"; }
warn() { echo "[WARN]  $*" >&2; }
die()  { echo "[ERROR] $*" >&2; exit 1; }

need_root() {
    if [[ $EUID -ne 0 ]]; then
        die "this script must be run as root (sudo)."
    fi
}

# load_env_file [ARGS...]
# Loads environment variables from a .env file. Caller passes "$@" so this
# function can scan for an explicit --env <path> override before falling back
# to ./.env in the current working directory.
#
# Precedence (low → high) once layered into install scripts:
#   defaults.env  <  .env file  <  pre-existing process env  <  CLI flags
#
# The function only sources; it does not export. Callers must reference the
# resulting variables explicitly (e.g. PORT="${ZISK_COORDINATOR_PORT:-...}").
load_env_file() {
    local env_file=""
    local prev=""
    for arg in "$@"; do
        if [[ "$prev" == "--env" ]]; then
            env_file="$arg"
            break
        fi
        prev="$arg"
    done
    if [[ -z "$env_file" && -f "./.env" ]]; then
        env_file="./.env"
    fi
    if [[ -n "$env_file" ]]; then
        [[ -f "$env_file" ]] || die "env file not found: ${env_file}"
        info "Loading environment from ${env_file}"
        # shellcheck disable=SC1090
        set -a; source "$env_file"; set +a
    fi
}

require_os() {
    local expected="$1"
    local actual
    actual="$(uname -s)"
    if [[ "$actual" != "$expected" ]]; then
        die "this script targets ${expected}; current OS is ${actual}. Use the matching install script instead."
    fi
}

# Validates that ${WORKSPACE_ROOT} points at a real ZisK workspace clone
# (Cargo.toml present). Call from any branch that needs to read source-tree
# files (cargo build, sample configs). Avoids confusing downstream errors
# when the script is shipped standalone without the surrounding repo.
require_workspace_root() {
    [[ -f "${WORKSPACE_ROOT}/Cargo.toml" ]] || die \
"workspace root not found at ${WORKSPACE_ROOT} (Cargo.toml missing).
       This script expects to run from a clone of the ZisK repository.
       To install from a standalone copy, pass --binary <path> and --config <path>."
}

# build_or_use_binary CARGO_PACKAGE
# If $BINARY_SRC is unset, builds CARGO_PACKAGE from the workspace and assigns
# the resulting binary path to $BINARY_SRC. Validates the binary exists.
# Reads/writes globals: BINARY_NAME, BINARY_SRC, WORKSPACE_ROOT.
build_or_use_binary() {
    local pkg="$1"
    if [[ -z "${BINARY_SRC}" ]]; then
        require_workspace_root
        info "Building ${BINARY_NAME} from source..."
        cargo build --release -p "${pkg}" --manifest-path "${WORKSPACE_ROOT}/Cargo.toml"
        BINARY_SRC="${WORKSPACE_ROOT}/target/release/${BINARY_NAME}"
    fi
    [[ -f "${BINARY_SRC}" ]] || die "binary not found at ${BINARY_SRC}"
}

# install_config_or_sample CONFIG_SRC CONFIG_DST GROUP SAMPLE_PATH
# Installs config with 0640 mode, owned by root:GROUP:
#   - If CONFIG_SRC is non-empty: install CONFIG_SRC to CONFIG_DST.
#   - Else if CONFIG_DST is missing: install SAMPLE_PATH (warn if sample missing).
#   - Else: leave CONFIG_DST unchanged.
install_config_or_sample() {
    local src="$1" dst="$2" group="$3" sample="$4"
    if [[ -n "${src}" ]]; then
        info "Installing config from ${src}..."
        install -m 640 -o root -g "${group}" "${src}" "${dst}"
    elif [[ ! -f "${dst}" ]]; then
        require_workspace_root
        info "Installing sample config to ${dst}..."
        if [[ -f "${sample}" ]]; then
            install -m 640 -o root -g "${group}" "${sample}" "${dst}"
        else
            warn "sample config not found at ${sample}; skipping."
        fi
    else
        info "Config already exists at ${dst}; leaving unchanged."
    fi
}

# darwin_create_service_user USER GROUP REAL_NAME HOME_DIR
# Creates a macOS system group + user via dscl, idempotently. Allocates the
# next-available GID/UID by scanning existing entries.
darwin_create_service_user() {
    local user="$1" group="$2" real_name="$3" home="$4"
    local gid uid

    if ! dscl . -read "/Groups/${group}" &>/dev/null; then
        info "Creating group '${group}'..."
        gid=$(( $(dscl . -list /Groups PrimaryGroupID | awk '{print $2}' | sort -n | tail -1) + 1 ))
        dscl . -create "/Groups/${group}"
        dscl . -create "/Groups/${group}" PrimaryGroupID "$gid"
        dscl . -create "/Groups/${group}" RecordName    "${group}"
    fi

    if ! dscl . -read "/Users/${user}" &>/dev/null; then
        info "Creating user '${user}'..."
        uid=$(( $(dscl . -list /Users UniqueID | awk '{print $2}' | sort -n | tail -1) + 1 ))
        gid=$(dscl . -read "/Groups/${group}" PrimaryGroupID | awk '{print $2}')
        dscl . -create "/Users/${user}"
        dscl . -create "/Users/${user}" UniqueID         "$uid"
        dscl . -create "/Users/${user}" PrimaryGroupID   "$gid"
        dscl . -create "/Users/${user}" UserShell        /usr/bin/false
        dscl . -create "/Users/${user}" RealName         "${real_name}"
        dscl . -create "/Users/${user}" NFSHomeDirectory "${home}"
    fi
}

# resolve_mpi_config MPIRUN_MISSING_HINT
# Resolves MPI parameters based on caller-set globals:
#   Reads:  MPI_ENABLED, MPI_NP, MPI_PPR, MPI_THREADS, COMMON_DIR
#   Writes: MPI_NP, MPI_PPR, MPI_THREADS, MPI_NUM_SOCKETS, MPI_NUM_GPUS,
#           MPI_TOTAL_THREADS, MPIRUN_BIN (only when MPI_ENABLED)
# Validates that --mpi-processes/--mpi-numa-ppr/--mpi-threads are all-or-none,
# rejects them under --no-mpi, runs auto-detect when no manual triplet was
# provided, and resolves mpirun's absolute path.
resolve_mpi_config() {
    local missing_hint="$1"
    local manual_count=0
    [[ -n "$MPI_NP" ]]      && manual_count=$((manual_count + 1))
    [[ -n "$MPI_PPR" ]]     && manual_count=$((manual_count + 1))
    [[ -n "$MPI_THREADS" ]] && manual_count=$((manual_count + 1))

    if ! $MPI_ENABLED; then
        [[ "$manual_count" -gt 0 ]] && die "--mpi-processes/--mpi-numa-ppr/--mpi-threads cannot be used with --no-mpi."
        return
    fi

    if [[ "$manual_count" -gt 0 && "$manual_count" -lt 3 ]]; then
        die "--mpi-processes, --mpi-numa-ppr and --mpi-threads must all be specified together."
    fi
    if [[ "$manual_count" -eq 0 ]]; then
        info "Auto-detecting MPI parameters..."
        local mpi_out
        mpi_out="$("${COMMON_DIR}/mpi_params.sh")" || die "MPI auto-detection failed."
        eval "$mpi_out"
        MPI_THREADS="${MPI_RAYON_NUM_THREADS}"
    fi

    MPIRUN_BIN="$(command -v mpirun || true)"
    [[ -z "$MPIRUN_BIN" ]] && die "mpirun not found in PATH. ${missing_hint}"
    info "Using mpirun: ${MPIRUN_BIN}"
}
