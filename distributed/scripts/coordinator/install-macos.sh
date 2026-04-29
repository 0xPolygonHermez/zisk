#!/usr/bin/env bash
# install-macos.sh — install zisk-coordinator as a launchd daemon on macOS.
#
# Usage:
#   sudo ./install-macos.sh [OPTIONS]
#
# Options:
#   --binary PATH    Use a pre-built binary instead of building from source
#   --config PATH    Install an existing coordinator.toml instead of the sample
#   --port N         Listening port (default: 7000, maps to --api-port)
#   --uninstall      Stop, unload, and remove the service and binary
#
# What this script does:
#   1. Verifies it's running on macOS
#   2. Creates the 'zisk-coordinator' system user via dscl
#   3. Builds or installs the binary to /usr/local/bin/zisk-coordinator
#   4. Installs config to /etc/zisk/coordinator.toml
#   5. Creates the /var/lib/zisk working directory
#   6. Writes /Library/LaunchDaemons/com.zisk.coordinator.plist
#   7. Writes /etc/newsyslog.d/zisk-coordinator.conf for log rotation
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

# ── argument parsing ──────────────────────────────────────────────────────────

BINARY_SRC=""
CONFIG_SRC=""
PORT="${DEFAULT_PORT}"
UNINSTALL=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --binary)    BINARY_SRC="$2";  shift 2 ;;
        --config)    CONFIG_SRC="$2";  shift 2 ;;
        --port)      PORT="$2";        shift 2 ;;
        --uninstall) UNINSTALL=true;   shift ;;
        *) die "Unknown option: $1" ;;
    esac
done

# ── uninstall ─────────────────────────────────────────────────────────────────

if $UNINSTALL; then
    need_root
    info "Uninstalling ${BINARY_NAME}..."
    launchctl unload "${LAUNCHD_PLIST}" 2>/dev/null || true
    rm -f "${LAUNCHD_PLIST}" "${BINARY_DST}" "${NEWSYSLOG_CONF}"
    info "Done. Config files under ${CONFIG_DIR}/ are left in place."
    info "Remove manually if no longer needed: sudo rm -rf ${CONFIG_DIR}"
    exit 0
fi

# ── install ───────────────────────────────────────────────────────────────────

need_root

# 1. Build or use pre-built binary
build_or_use_binary "zisk-coordinator-server"

# 2. Create system group + user via dscl
darwin_create_service_user "${SERVICE_USER}" "${SERVICE_GROUP}" "ZisK Coordinator" "/var/empty"

# 3. Install binary
info "Installing binary to ${BINARY_DST}..."
install -m 755 -o root -g wheel "${BINARY_SRC}" "${BINARY_DST}"

# 4. Install config
mkdir -p "${CONFIG_DIR}"
install_config_or_sample "${CONFIG_SRC}" "${CONFIG_DST}" "${SERVICE_GROUP}" \
    "${WORKSPACE_ROOT}/distributed/crates/coordinator-server/config/coordinator.example.toml"

# 5. Create working and log directories
mkdir -p "${WORK_DIR}" "${LOG_DIR}"
chown "${SERVICE_USER}:${SERVICE_GROUP}" "${WORK_DIR}" "${LOG_DIR}"

# 6. Write launchd plist
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
        <string>${BINARY_DST}</string>
        <string>--config</string>
        <string>${CONFIG_DST}</string>
        <string>-p</string>
        <string>${PORT}</string>
    </array>

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

# 8. Load the service
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
