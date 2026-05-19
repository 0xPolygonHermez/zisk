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
#   --api-port N       Client-facing gRPC API port (optional; TOML default if unset)
#   --cluster-port N   Worker-facing gRPC port (optional; TOML default if unset)
#   --metrics-port N   Prometheus metrics port (optional; TOML default if unset)
#   --log-level LEVEL  trace | debug | info | warn | error (optional; RUST_LOG)
#   --no-service       Container mode: install binary/config/state only;
#                      skip service user creation and service manager setup.
#   --no-start         Linux: enable but don't start.
#                      macOS: write plist but don't load (same as --no-enable).
#   --no-enable        Linux: install unit but don't enable or start.
#                      macOS: write plist but don't load.
#   --uninstall        Stop, disable, and remove the service
#   -y, --yes          Skip uninstall confirmation
#
# Env-var equivalents (CLI flags win): ZISK_COORDINATOR_BINARY,
# ZISK_COORDINATOR_CONFIG, ZISK_COORDINATOR_API_PORT,
# ZISK_COORDINATOR_CLUSTER_PORT, ZISK_COORDINATOR_METRICS_PORT, RUST_LOG.
#
# ── file layout (where install.sh puts things) ───────────────────────────────
# Coordinator caches registered guest ELFs under WORK_DIR/cache/ (content-
# addressed by blake3 hash); ZISK_CACHE_DIR is exported in the unit/plist so
# ZiskPaths resolves the cache there instead of falling back to $HOME/.zisk
# (which lands in /var/empty for the system user). It does not touch the
# proving key or rom-setup, so ZISK_HOME is left unset. The bundle is still
# populated via ziskup --system at install time so the 'zisk' system group
# exists and a co-located worker can share it without re-bootstrapping.
#
# Linux service mode:
#   Binary               /usr/local/bin/zisk-coordinator
#   Config               /etc/zisk/coordinator.toml
#   Systemd unit         /etc/systemd/system/zisk-coordinator.service
#   Logs                 journald (no on-disk log dir)
#   Service user         zisk-coordinator:zisk-coordinator (+ supplementary 'zisk')
#   State (writable)     /var/lib/zisk-coordinator/         (cache/ for registered ELFs)
#   Bundle (read-only)   /opt/zisk/                         (populated for parity; unused at runtime)
#
# macOS service mode (bundle root differs — FHS doesn't apply on macOS):
#   Binary               /usr/local/bin/zisk-coordinator
#   Config               /etc/zisk/coordinator.toml
#   launchd plist        /Library/LaunchDaemons/com.zisk.coordinator.plist
#   Log file             /var/log/zisk-coordinator/zisk-coordinator.log  (rotated by newsyslog)
#   newsyslog config     /etc/newsyslog.d/zisk-coordinator.conf
#   State (writable)     /usr/local/var/zisk-coordinator/   (cache/ for registered ELFs; mac /var/lib trips SIP)
#   Bundle (read-only)   /Library/Application Support/ZisK/

# ── self-bootstrap (curl-pipe-able install) ──────────────────────────────────
# When this script runs without its sibling files (curl | bash, or copied
# without lib.sh/defaults.env/ziskup nearby), download the deploy tree from
# GitHub and re-exec from the temp copy.
#
# Usage from a fresh server (no clone needed):
#
#   curl -fsL https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/distributed/deploy/scripts/coordinator/install.sh \
#       | sudo bash -s -- --api-port 7000
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
        "${BOOTSTRAP_TMP}/distributed/deploy/scripts/coordinator" \
        "${BOOTSTRAP_TMP}/distributed/crates/coordinator-server/config" \
        "${BOOTSTRAP_TMP}/ziskup"
    for f in \
        distributed/deploy/scripts/common/lib.sh \
        distributed/deploy/scripts/coordinator/install.sh \
        distributed/deploy/scripts/coordinator/defaults.env \
        distributed/crates/coordinator-server/config/coordinator.example.toml \
        ziskup/ziskup ; do
        if ! curl -fsL --retry 3 --max-time 30 "${BASE}/${f}" -o "${BOOTSTRAP_TMP}/${f}"; then
            echo "[bootstrap] failed to download ${f} from ${BRANCH}" >&2
            rm -rf "${BOOTSTRAP_TMP}"
            exit 1
        fi
    done
    chmod +x "${BOOTSTRAP_TMP}/distributed/deploy/scripts/coordinator/install.sh" \
             "${BOOTSTRAP_TMP}/ziskup/ziskup"
    # Clean up the bootstrap dir on exit (success, failure, or interrupt).
    trap 'rm -rf "${BOOTSTRAP_TMP}"' EXIT
    bash "${BOOTSTRAP_TMP}/distributed/deploy/scripts/coordinator/install.sh" "$@"
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

print_banner coordinator

# ── load .env (if any), then argument parsing ─────────────────────────────────

load_env_file "$@"

BINARY_SRC="${ZISK_COORDINATOR_BINARY:-}"
CONFIG_SRC="${ZISK_COORDINATOR_CONFIG:-}"
API_PORT="${ZISK_COORDINATOR_API_PORT:-}"
CLUSTER_PORT="${ZISK_COORDINATOR_CLUSTER_PORT:-}"
METRICS_PORT="${ZISK_COORDINATOR_METRICS_PORT:-}"
LOG_LEVEL="${RUST_LOG:-}"
NO_SERVICE=false
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
        --no-service)    NO_SERVICE=true;      shift ;;
        --no-start)      NO_START=true;      shift ;;
        --no-enable)     NO_ENABLE=true;     shift ;;
        --uninstall)     UNINSTALL=true;     shift ;;
        -y|--yes)        ASSUME_YES=true;    shift ;;
        *) die "Unknown option: $1" ;;
    esac
done

# ── uninstall ─────────────────────────────────────────────────────────────────

if $UNINSTALL; then
    $NO_SERVICE && die "--uninstall is not supported with --no-service (no service unit/plist is installed)."
    uninstall_service
    exit 0
fi

# ── install ───────────────────────────────────────────────────────────────────

need_root

if $NO_SERVICE; then
    CURRENT_USER="$(id -un)"
    CURRENT_GROUP="$(id -gn)"
    INSTALL_CONFIG_GROUP="${CURRENT_GROUP}"
    INSTALL_OWNER="${CURRENT_USER}:${CURRENT_GROUP}"
else
    INSTALL_CONFIG_GROUP="${SERVICE_GROUP}"
    INSTALL_OWNER="${SERVICE_USER}:${SERVICE_GROUP}"
fi

# 1. Populate the shared ZisK bundle at ${BUNDLE_DIR} via ziskup. Idempotent —
# if worker install.sh ran first on this host, this is a near-no-op.
ZISKUP_BIN="$(resolve_ziskup_bin)"
if $NO_SERVICE; then
    ZISKUP_ARGS=(--system --prefix "${BUNDLE_DIR}" --owner "${INSTALL_OWNER}" --yes --nokey)
else
    ZISKUP_ARGS=(--system --prefix "${BUNDLE_DIR}" --owner zisk:zisk --yes --nokey)
fi
info "Populating ${BUNDLE_DIR} via ${ZISKUP_BIN} ${ZISKUP_ARGS[*]}..."
"${ZISKUP_BIN}" "${ZISKUP_ARGS[@]}"

# 2. Resolve the zisk-coordinator binary (from --binary or from the bundle).
resolve_service_binary

# 3. Create system group + user (with 'zisk' supplementary so it can read the
# bundle, in case future coordinator versions need toolchain payload).
if ! $NO_SERVICE; then
    create_service_user "${SERVICE_USER}" "${SERVICE_GROUP}" "ZisK Coordinator" "/var/empty"
    add_user_to_group "${SERVICE_USER}" zisk
fi

# 4. Install binary
install_binary "${BINARY_SRC}" "${BINARY_DST}"

# 5. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${INSTALL_CONFIG_GROUP}" \
    "${DEPLOY_ROOT}/crates/coordinator-server/config/coordinator.example.toml"

# 6. Create working (and log on macOS) directories. cache/ holds the ELF
# registry written by register_guest_program.
if [[ "$OS_NAME" == "Darwin" ]]; then
    mkdir -p "${WORK_DIR}" "${WORK_DIR}/cache" "${LOG_DIR}"
    chown -R "${INSTALL_OWNER}" "${WORK_DIR}" "${LOG_DIR}"
else
    mkdir -p "${WORK_DIR}" "${WORK_DIR}/cache"
    chown -R "${INSTALL_OWNER}" "${WORK_DIR}"
fi

if $NO_SERVICE; then
    RUN_CMD=("${BINARY_DST}" --config "${CONFIG_DST}")
    [[ -n "${API_PORT}" ]] && RUN_CMD+=(--api-port "${API_PORT}")
    [[ -n "${CLUSTER_PORT}" ]] && RUN_CMD+=(--cluster-port "${CLUSTER_PORT}")
    [[ -n "${METRICS_PORT}" ]] && RUN_CMD+=(--metrics-port "${METRICS_PORT}")
    [[ -n "${LOG_LEVEL}" ]] && RUN_CMD+=(--log-level "${LOG_LEVEL}")

    echo
    info "✓ ${BINARY_NAME} installed in --no-service mode."
    info "Run it directly in this container:"
    echo "  export ZISK_CACHE_DIR=${WORK_DIR}/cache"
    echo "  ${RUN_CMD[*]}"
    exit 0
fi

# 7. Write service unit
if [[ "$OS_NAME" == "Darwin" ]]; then
    info "Writing plist to ${LAUNCHD_PLIST}..."

    build_program_args() {
        printf '        <string>%s</string>\n' "${BINARY_DST}"
        printf '        <string>--config</string>\n'
        printf '        <string>%s</string>\n' "${CONFIG_DST}"
        if [[ -n "$API_PORT" ]]; then
            printf '        <string>--api-port</string>\n'
            printf '        <string>%s</string>\n' "${API_PORT}"
        fi
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

    <key>EnvironmentVariables</key>
    <dict>
        <key>ZISK_CACHE_DIR</key>
        <string>${WORK_DIR}/cache</string>
    </dict>

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

    # 8. Write newsyslog rotation config
    info "Writing newsyslog config to ${NEWSYSLOG_CONF}..."
    cat > "${NEWSYSLOG_CONF}" <<NEWSYSLOG
# ${BINARY_NAME} log rotation — max ${LOG_MAX_SIZE_MB}MB per file, keep ${LOG_ROTATIONS} rotations, gzipped
${LOG_DIR}/${BINARY_NAME}.log  ${SERVICE_USER}:${SERVICE_GROUP}  640  ${LOG_ROTATIONS}  $(( LOG_MAX_SIZE_MB * 1024 ))  *  JG
NEWSYSLOG
    chmod 0644 "${NEWSYSLOG_CONF}"
else
    # Build ExecStart line — required flags first, optional appended only when set
    EXEC_START="ExecStart=${BINARY_DST} --config ${CONFIG_DST}"
    [[ -n "$API_PORT" ]]     && EXEC_START+=" --api-port ${API_PORT}"
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
Environment=ZISK_CACHE_DIR=${WORK_DIR}/cache

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
# ${BINARY_NAME}:CONFIG_FILE=${CONFIG_DST}
# ${BINARY_NAME}:SVC_USER=${SERVICE_USER}
# ${BINARY_NAME}:SVC_GROUP=${SERVICE_GROUP}
EOF
fi

# 9. Activate service and (if started) print management hints
activate_service
if $SHOW_HINTS; then
    print_post_install_hints "${BASH_SOURCE[0]}"
fi
