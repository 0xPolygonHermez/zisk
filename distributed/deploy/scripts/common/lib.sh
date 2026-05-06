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
#     ZISK_WORKER_PROVING_KEY_SNARK  SNARK proving key dir     (--proving-key-snark)
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

# bundle_dir_for_os
# Echoes the platform-specific install root for the read-only ZisK bundle:
#   Linux  → /opt/zisk
#   Darwin → /Library/Application Support/ZisK
# This is the path ziskup --system writes to, where worker (and future
# coordinator bundle consumers) read bin/, zisk/, provingKey/ from.
bundle_dir_for_os() {
    case "$OS_NAME" in
        Linux)  echo "/opt/zisk" ;;
        Darwin) echo "/Library/Application Support/ZisK" ;;
        *)      die "bundle_dir_for_os: unsupported OS '${OS_NAME}'" ;;
    esac
}

# ensure_zisk_user_group BUNDLE_DIR
# Creates the shared 'zisk' system user and group if missing. Sets the user's
# home dir to BUNDLE_DIR. Idempotent. Branches Linux/Darwin.
ensure_zisk_user_group() {
    local bundle="$1"
    if [[ "$OS_NAME" == "Darwin" ]]; then
        local gid uid
        if ! dscl . -read /Groups/zisk &>/dev/null; then
            info "Creating system group 'zisk'..."
            gid=$(( $(dscl . -list /Groups PrimaryGroupID | awk '{print $2}' | sort -n | tail -1) + 1 ))
            dscl . -create /Groups/zisk
            dscl . -create /Groups/zisk PrimaryGroupID "$gid"
            dscl . -create /Groups/zisk RecordName    zisk
        fi
        if ! dscl . -read /Users/zisk &>/dev/null; then
            info "Creating system user 'zisk'..."
            uid=$(( $(dscl . -list /Users UniqueID | awk '{print $2}' | sort -n | tail -1) + 1 ))
            gid=$(dscl . -read /Groups/zisk PrimaryGroupID | awk '{print $2}')
            dscl . -create /Users/zisk
            dscl . -create /Users/zisk UniqueID         "$uid"
            dscl . -create /Users/zisk PrimaryGroupID   "$gid"
            dscl . -create /Users/zisk UserShell        /usr/bin/false
            dscl . -create /Users/zisk RealName         "ZisK Bundle"
            dscl . -create /Users/zisk NFSHomeDirectory "${bundle}"
        fi
    else
        if ! getent group zisk &>/dev/null; then
            info "Creating system group 'zisk'..."
            groupadd --system zisk
        fi
        if ! id zisk &>/dev/null; then
            info "Creating system user 'zisk'..."
            useradd --system --gid zisk -d "${bundle}" -s /usr/sbin/nologin zisk
        fi
    fi
}

# add_user_to_group USER GROUP
# Adds an existing user to an existing supplementary group. Idempotent.
add_user_to_group() {
    local user="$1" group="$2"
    if [[ "$OS_NAME" == "Darwin" ]]; then
        if ! dseditgroup -o checkmember -m "$user" "$group" &>/dev/null; then
            info "Adding ${user} to group ${group}..."
            dseditgroup -o edit -a "$user" -t user "$group"
        fi
    else
        if ! id -nG "$user" 2>/dev/null | tr ' ' '\n' | grep -qx "$group"; then
            info "Adding ${user} to group ${group}..."
            usermod -aG "$group" "$user"
        fi
    fi
}

# print_banner SERVICE
# Identifies ZisK and the service being installed. Called at the top of each
# install script so operators see what they're running and where to find the
# project before any sudo prompt or build output scrolls past.
print_banner() {
    local service="$1"
    cat <<BANNER
============================================================
  ZisK — distributed proving system  (${service} installer)
  https://github.com/0xPolygonHermez/zisk
============================================================
BANNER
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

# resolve_service_binary
# Resolves $BINARY_SRC for the install: prefer an explicit --binary path,
# otherwise pick the one from the shared bundle (${BUNDLE_DIR}/bin/${BINARY_NAME})
# which ziskup --system has populated by the time this runs.
# Reads/writes globals: BINARY_NAME, BINARY_SRC, BUNDLE_DIR.
resolve_service_binary() {
    if [[ -z "${BINARY_SRC}" ]]; then
        BINARY_SRC="${BUNDLE_DIR}/bin/${BINARY_NAME}"
    fi
    [[ -f "${BINARY_SRC}" ]] || die "binary not found at ${BINARY_SRC} \
(populate the bundle via 'ziskup --system' first, or pass --binary <path>)"
}

# resolve_ziskup_bin
# Locates the ziskup script via a 3-level fallback:
#   1. ${WORKSPACE_ROOT}/ziskup/ziskup — workspace clone (dev case). Wins over
#                                        PATH so a stale ~/.zisk/bin/ziskup
#                                        from a prior user-mode install never
#                                        overrides edits made in the workspace.
#                                        install.sh self-bootstraps from
#                                        GitHub when there's no workspace, so
#                                        seeing one means "you're developing".
#   2. ziskup on PATH                  — operator-installed ziskup on a server
#                                        with no workspace clone (also lets
#                                        tests inject a stub via PATH).
#   3. ${BUNDLE_DIR}/bin/ziskup        — already-installed bundle (last resort;
#                                        may lag this branch if the latest
#                                        release predates new ziskup features)
# Echoes the resolved path; dies if none found.
resolve_ziskup_bin() {
    if [[ -x "${WORKSPACE_ROOT}/ziskup/ziskup" ]]; then
        echo "${WORKSPACE_ROOT}/ziskup/ziskup"
    elif command -v ziskup >/dev/null 2>&1; then
        command -v ziskup
    elif [[ -x "${BUNDLE_DIR}/bin/ziskup" ]]; then
        echo "${BUNDLE_DIR}/bin/ziskup"
    else
        die "ziskup not found at ${WORKSPACE_ROOT}/ziskup/ziskup, on PATH, or in ${BUNDLE_DIR}/bin/"
    fi
}

# bundle_meta_get KEY [BUNDLE_DIR]
# Read a single key=value field from ${BUNDLE_DIR}/.zisk-bundle. Echoes the
# value (empty if file or key is absent). The bundle file is the public
# metadata contract written by ziskup — see schema header in ziskup/ziskup.
bundle_meta_get() {
    local key="$1" bundle_dir="${2:-${BUNDLE_DIR}}"
    local file="${bundle_dir}/.zisk-bundle"
    [[ -f "$file" ]] || return 0
    awk -F= -v k="$key" '$1 == k { sub(/^[^=]+=/, ""); print; exit }' "$file"
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
        if [[ -f "${sample}" ]]; then
            info "Installing sample config to ${dst}..."
            install -m 640 -o root -g "${group}" "${sample}" "${dst}"
        else
            warn "sample config not found at ${sample}; pass --config or place a config at ${dst}."
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
            local useradd_args=(--system --gid "$group" --no-create-home --shell /usr/sbin/nologin)
            [[ -n "$home" ]] && useradd_args+=(--home-dir "$home")
            useradd "${useradd_args[@]}" "$user"
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
        if [[ "$manual_count" -gt 0 ]]; then
            die "--mpi-processes/--mpi-numa-ppr/--mpi-threads cannot be used with --no-mpi."
        fi
        return 0
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
#   systemd: trailing "# zisk-coordinator:DATA_DIR=/var/lib/zisk-coordinator" lines
#   launchd: trailing "<!-- zisk-coordinator:DATA_DIR=... -->" lines
#
# Pass --yes / -y to skip that prompt and uninstall immediately.

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
        # Literal index() rather than `$0 ~ m`: BINARY_NAME is interpolated into
        # the marker, so a future name with regex metachars (`. + * ( [`) would
        # otherwise silently misparse this file at uninstall time.
        awk -v m="# ${marker}" \
            'index($0,m) == 1 { print substr($0, length(m) + 1); exit }' \
            "$file"
    fi
}

# remove_dir DIR LABEL
# Removes DIR. No-op if DIR doesn't exist.
remove_dir() {
    local dir="$1" label="$2"
    [[ -n "$dir" && -d "$dir" ]] || return 0
    rm -rf "$dir"
    info "Removed ${label} ${dir}."
}

# remove_config_file FILE DIR
# Removes a service's config file, then rmdirs DIR if it's now
# empty (silent on non-empty — leaves sibling services' configs intact).
# No-ops if FILE doesn't exist.
remove_config_file() {
    local file="$1" dir="$2"
    [[ -n "$file" && -f "$file" ]] || return 0
    rm -f "$file"
    info "Removed ${file}."
    if [[ -n "$dir" && -d "$dir" ]] && rmdir "$dir" 2>/dev/null; then
        info "Removed empty ${dir}."
    fi
}

# remove_user_group USER GROUP
# Removes a system user + group. Branches on OS.
remove_user_group() {
    local user="$1" group="$2"
    [[ -n "$user" && -n "$group" ]] || return 0
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

    local data_dir log_dir config_dir config_file svc_user svc_group
    data_dir="$(read_unit_metadata "$marker" DATA_DIR)"
    log_dir="$(read_unit_metadata "$marker" LOG_DIR)"
    config_dir="$(read_unit_metadata "$marker" CONFIG_DIR)"
    config_file="$(read_unit_metadata "$marker" CONFIG_FILE)"
    svc_user="$(read_unit_metadata "$marker" SVC_USER)"
    svc_group="$(read_unit_metadata "$marker" SVC_GROUP)"
    : "${data_dir:=$WORK_DIR}"
    : "${log_dir:=$LOG_DIR}"
    : "${config_dir:=$CONFIG_DIR}"
    : "${config_file:=$CONFIG_DST}"
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

    # Drop the service user/group FIRST, then dirs. On macOS, dirs registered
    # as a user's home (NFSHomeDirectory) acquire ACLs that block rm -rf until
    # the dscl record is gone. Linux is order-insensitive.
    remove_user_group "${svc_user}" "${svc_group}"
    remove_dir "${log_dir}" "log directory"
    remove_dir "${data_dir}" "data directory"
    # config_dir is shared between services (worker + coordinator both keep
    # their .toml under /etc/zisk). Remove only this service's own config
    # file, then rmdir the parent — succeeds silently iff the dir is empty
    # so a sibling service's config is never collateral damage.
    remove_config_file "${config_file}" "${config_dir}"

    info "${BINARY_NAME} uninstalled."

    # The shared ZisK bundle (/opt/zisk on Linux, /Library/Application Support/ZisK
    # on macOS) and the 'zisk' system user/group are NOT touched here — they
    # may still be in use by other ZisK services on this host. Tell the
    # operator how to clean them up when they're truly done with ZisK.
    echo
    info "The shared ZisK bundle and 'zisk' system user remain on this host."
    echo "  To remove them (only when no other ZisK service is installed):"
    echo "    sudo ziskup --uninstall --system"
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
            # `enable + restart` (not `enable --now`): on a re-install the unit
            # is already active and `--now`'s implicit `start` is a no-op, so
            # the running process keeps the old text segment from the prior
            # /usr/local/bin/<binary>. `restart` reloads the just-installed
            # binary unconditionally; on a fresh install it's equivalent to
            # `start`.
            systemctl enable "${BINARY_NAME}"
            systemctl restart "${BINARY_NAME}"
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
