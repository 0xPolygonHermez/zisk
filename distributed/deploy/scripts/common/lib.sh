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
#     ZISK_COORDINATOR_BINARY        pre-built binary path     (--binary)
#     ZISK_COORDINATOR_CONFIG        TOML config path          (--config)
#     ZISK_COORDINATOR_API_PORT      client-facing API port    (--api-port)
#     ZISK_COORDINATOR_CLUSTER_PORT  worker-facing port        (--cluster-port)
#     ZISK_COORDINATOR_METRICS_PORT  Prometheus metrics port   (--metrics-port)
#
#   Worker
#     ZISK_WORKER_BINARY             pre-built binary path     (--binary)
#     ZISK_WORKER_CONFIG             TOML config path          (--config)
#     ZISK_WORKER_PROVING_KEY        proving key directory     (--proving-key)
#     ZISK_WORKER_COORDINATOR_URL    coordinator gRPC URL      (--coordinator-url)
#     ZISK_WORKER_ID                 worker identifier         (--worker-id)
#     ZISK_WORKER_COMPUTE_CAPACITY   advertised compute units  (--compute-capacity)
#     ZISK_WORKER_EMULATOR           true|false                (--emulator)
#     ZISK_WORKER_ASM                ASM file path             (--asm)
#     ZISK_WORKER_GPU                true|false                (--gpu)
#     ZISK_WORKER_MPI                true|false                (--mpi / --no-mpi)
#     ZISK_WORKER_MPI_PROCESSES      -np override              (--mpi-processes)
#     ZISK_WORKER_MPI_NUMA_PPR       ppr:N:numa override       (--mpi-numa-ppr)
#     ZISK_WORKER_MPI_THREADS        RAYON_NUM_THREADS         (--mpi-threads)
#
#   Shared
#     RUST_LOG                       log level (trace..error)  (--log-level)

info() { echo "[INFO]  $*"; }
warn() { echo "[WARN]  $*" >&2; }
die()  { echo "[ERROR] $*" >&2; exit 1; }

# Detected once on source. Used to branch service-management, user creation,
# binary install, and uninstall helpers. Anything other than Linux/Darwin is
# rejected here so callers don't need to repeat the guard.
OS_NAME="$(uname -s)"
[[ "$OS_NAME" == "Linux" || "$OS_NAME" == "Darwin" ]] || \
    die "lib.sh: unsupported OS '${OS_NAME}'. Only Linux and Darwin are supported."

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
# resulting variables explicitly (e.g. PORT="${ZISK_COORDINATOR_API_PORT:-...}").
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

# create_service_user USER GROUP REAL_NAME [HOME_DIR]
# Creates a system group and a system user, idempotently. Branches Linux/Darwin.
# HOME_DIR is required on Darwin (passed to NFSHomeDirectory) and ignored on
# Linux (where we use --no-create-home). On Darwin the next-available GID/UID
# is allocated by scanning existing dscl entries.
create_service_user() {
    local user="$1" group="$2" real_name="$3" home="${4:-}"
    if [[ "$OS_NAME" == "Darwin" ]]; then
        [[ -n "$home" ]] || die "create_service_user: HOME_DIR required on Darwin"
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
    else
        if ! getent group "$group" &>/dev/null; then
            info "Creating system group '${group}'..."
            groupadd --system "$group"
        fi
        if ! id "$user" &>/dev/null; then
            info "Creating system user '${user}'..."
            useradd --system --gid "$group" --no-create-home --shell /usr/sbin/nologin "$user"
        fi
    fi
}

# install_binary SRC DST
# Installs SRC to DST with mode 0755 and root ownership. Branches OS for the
# group: root:root on Linux, root:wheel on Darwin.
install_binary() {
    local src="$1" dst="$2"
    info "Installing binary to ${dst}..."
    if [[ "$OS_NAME" == "Darwin" ]]; then
        install -m 755 -o root -g wheel "$src" "$dst"
    else
        install -m 755 "$src" "$dst"
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

# ── uninstall helpers ────────────────────────────────────────────────────────
#
# Install scripts embed a metadata footer in the generated systemd unit (or
# launchd plist) at install time. Uninstall reads it back so the cleanup uses
# the paths actually installed, even if defaults.env has since changed.
#
#   systemd: trailing "# zisk-coordinator:DATA_DIR=/var/lib/zisk" lines
#   launchd: trailing "<!-- zisk-coordinator:DATA_DIR=... -->" lines
#
# Set ASSUME_YES=true (e.g. via --yes / -y) to skip every prompt below.

# confirm PROMPT [DEFAULT]
# Y/N prompt. DEFAULT is "y" or "n" (default "n"). Returns 0 on yes, 1 on no.
# When ASSUME_YES=true, returns 0 without reading.
confirm() {
    local prompt="$1" default="${2:-n}" reply
    if [[ "${ASSUME_YES:-false}" == "true" ]]; then
        return 0
    fi
    if [[ "$default" == "y" ]]; then
        read -r -p "${prompt} [Y/n] " reply
        reply="${reply:-y}"
    else
        read -r -p "${prompt} [y/N] " reply
        reply="${reply:-n}"
    fi
    [[ "$(echo "$reply" | tr '[:upper:]' '[:lower:]')" == "y" ]]
}

# read_unit_metadata FILE KEY
# Reads a metadata value for ${BINARY_NAME} embedded in FILE (systemd unit or
# launchd plist). Prints the value to stdout; empty if FILE missing or key absent.
read_unit_metadata() {
    local file="$1" key="$2"
    [[ -f "$file" ]] || return 0
    local marker="${BINARY_NAME}:${key}="
    if [[ "$file" == *.plist ]]; then
        awk -v m="<!-- ${marker}" -v e=" -->" \
            'index($0,m) { s=index($0,m)+length(m); print substr($0,s,index($0,e)-s); exit }' \
            "$file"
    else
        awk -v m="^# ${marker}" \
            '$0 ~ m { sub(m,""); print; exit }' \
            "$file"
    fi
}

# prompt_remove_dir DIR LABEL
# Prompts to remove DIR (default no). No-op if DIR doesn't exist.
prompt_remove_dir() {
    local dir="$1" label="$2"
    [[ -n "$dir" && -d "$dir" ]] || return 0
    if confirm "Remove ${label} '${dir}'?"; then
        rm -rf "$dir"
        info "Removed ${dir}."
    fi
}

# prompt_remove_user_group USER GROUP
# Prompts to remove a system user + group (default no). Branches on OS.
prompt_remove_user_group() {
    local user="$1" group="$2"
    [[ -n "$user" && -n "$group" ]] || return 0
    confirm "Remove system user '${user}' and group '${group}'?" || return 0
    if [[ "$OS_NAME" == "Darwin" ]]; then
        if dscl . -read "/Users/${user}" &>/dev/null; then
            dscl . -delete "/Users/${user}" && info "Removed user '${user}'."
        fi
        if dscl . -read "/Groups/${group}" &>/dev/null; then
            dscl . -delete "/Groups/${group}" && info "Removed group '${group}'."
        fi
    else
        if id "$user" &>/dev/null; then
            if userdel "$user" 2>/dev/null; then
                info "Removed user '${user}'."
            else
                warn "Could not remove user '${user}' (may have running processes)."
            fi
        fi
        if getent group "$group" &>/dev/null; then
            if groupdel "$group" 2>/dev/null; then
                info "Removed group '${group}'."
            else
                warn "Could not remove group '${group}'."
            fi
        fi
    fi
}

# ── service lifecycle (install/uninstall/activate/post-install hints) ─────────
#
# These read service-identity globals from the caller's defaults.env:
#   BINARY_NAME, BINARY_DST, UNIT_FILE, LAUNCHD_PLIST, LAUNCHD_LABEL,
#   NEWSYSLOG_CONF, WORK_DIR, LOG_DIR, CONFIG_DIR, SERVICE_USER, SERVICE_GROUP

# uninstall_service
# Stops + removes the service, then prompts for cleanup of dirs and svc user.
# Recovers install-time paths from the unit/plist metadata footer; falls back
# to the caller's defaults.env globals if metadata is missing (lets pre-Phase-2
# installs still uninstall cleanly).
uninstall_service() {
    need_root

    local marker
    if [[ "$OS_NAME" == "Darwin" ]]; then
        marker="${LAUNCHD_PLIST}"
    else
        marker="${UNIT_FILE}"
    fi
    [[ -f "$marker" ]] || die "${BINARY_NAME} is not installed (${marker} not found)."
    confirm "Uninstall ${BINARY_NAME}?" || { info "Cancelled."; exit 0; }

    local data_dir log_dir config_dir svc_user svc_group
    data_dir="$(read_unit_metadata "$marker" DATA_DIR)"
    log_dir="$(read_unit_metadata "$marker" LOG_DIR)"
    config_dir="$(read_unit_metadata "$marker" CONFIG_DIR)"
    svc_user="$(read_unit_metadata "$marker" SVC_USER)"
    svc_group="$(read_unit_metadata "$marker" SVC_GROUP)"
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
}

# activate_service
# Starts/enables (or skips) the freshly-installed service. Reads NO_ENABLE and
# NO_START booleans from the caller. Sets SHOW_HINTS=true on full activation,
# false otherwise (caller branches on it to print management hints).
activate_service() {
    SHOW_HINTS=false
    if [[ "$OS_NAME" == "Darwin" ]]; then
        if $NO_ENABLE || $NO_START; then
            local flag
            flag=$($NO_ENABLE && echo "--no-enable" || echo "--no-start")
            info "Plist installed. Service not loaded (${flag})."
            info "To load: sudo launchctl load -w ${LAUNCHD_PLIST}"
        else
            info "Loading service via launchctl..."
            launchctl unload "${LAUNCHD_PLIST}" 2>/dev/null || true
            launchctl load -w "${LAUNCHD_PLIST}"
            SHOW_HINTS=true
        fi
    else
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
            SHOW_HINTS=true
        fi
    fi
}

# print_post_install_hints CALLER_PATH
# Prints management hints. Caller passes ${BASH_SOURCE[0]} so the "Uninstall"
# hint references the actual script the user invoked.
print_post_install_hints() {
    local caller="$1"
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
    echo "  Uninstall: sudo $(basename "$caller") --uninstall"
}
