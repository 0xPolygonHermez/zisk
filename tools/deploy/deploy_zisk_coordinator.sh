#!/usr/bin/env bash
# =============================================================================
# deploy_zisk_coordinator.sh
#
# Deploys the zisk-coordinator service (install/uninstall).
#
# Variables can be set via CLI flags or environment variables.
# CLI flags take precedence over environment variables.
# =============================================================================
set -euo pipefail

OS="$(uname -s)"
SCRIPT_NAME="$(basename "$0")"
COORDINATOR_BIN_NAME="zisk-coordinator"

source "$(dirname "$0")/deploy_utils.sh"

# =============================================================================
# Defaults
# =============================================================================
DEFAULT_COORDINATOR_GROUP="zisk"
DEFAULT_COORDINATOR_USER="zisk"
DEFAULT_DATA_DIR="/var/lib/${COORDINATOR_BIN_NAME}"
DEFAULT_LOG_DIR="/var/log/${COORDINATOR_BIN_NAME}"
DEFAULT_PORT="7000"
# macOS-only log settings
DEFAULT_LOG_MAX_SIZE_MB="100"
DEFAULT_LOG_ROTATIONS="5"

# =============================================================================
# Resolve values: env vars override defaults, CLI flags override env vars
# =============================================================================
COORDINATOR_GROUP="${ZISK_COORDINATOR_GROUP:-$DEFAULT_COORDINATOR_GROUP}"
COORDINATOR_USER="${ZISK_COORDINATOR_USER:-$DEFAULT_COORDINATOR_USER}"
DATA_DIR="${ZISK_COORDINATOR_DATA_DIR:-$DEFAULT_DATA_DIR}"
LOG_DIR="${ZISK_COORDINATOR_LOG_DIR:-$DEFAULT_LOG_DIR}"
COORDINATOR_BIN="${ZISK_COORDINATOR_BIN:-}"
PORT="${ZISK_COORDINATOR_PORT:-$DEFAULT_PORT}"
EXTRA_ARGS="${ZISK_COORDINATOR_EXTRA_ARGS:-}"
# macOS-only log settings (ignored on Linux)
LOG_MAX_SIZE_MB="${ZISK_COORDINATOR_LOG_MAX_SIZE_MB:-$DEFAULT_LOG_MAX_SIZE_MB}"
LOG_ROTATIONS="${ZISK_COORDINATOR_LOG_ROTATIONS:-$DEFAULT_LOG_ROTATIONS}"

# =============================================================================
# Helpers (service-specific usage/validation)
# =============================================================================
usage_short() {
  cat <<EOF
Usage: $SCRIPT_NAME <command> [OPTIONS]

COMMANDS:
  install     Install and start the ${COORDINATOR_BIN_NAME} service
  uninstall   Stop and remove the ${COORDINATOR_BIN_NAME} service

Run '$SCRIPT_NAME install --help' for install options.

EXAMPLES:
  sudo ./$SCRIPT_NAME install --coordinator-bin /opt/zisk/bin/zisk-coordinator
  sudo ./$SCRIPT_NAME uninstall
EOF
  exit 0
}

usage_install() {
  cat <<EOF
Usage: $SCRIPT_NAME install [OPTIONS]

Install the ${COORDINATOR_BIN_NAME} service.

OPTIONS:
  --coordinator-group GROUP   System group name               (env: ZISK_COORDINATOR_GROUP, default: $DEFAULT_COORDINATOR_GROUP)
  --coordinator-user USER     System user name                (env: ZISK_COORDINATOR_USER, default: $DEFAULT_COORDINATOR_USER)
  --data-dir DIR              Data directory                  (env: ZISK_COORDINATOR_DATA_DIR, default: $DEFAULT_DATA_DIR)
  --log-dir DIR               Log directory                   (env: ZISK_COORDINATOR_LOG_DIR, default: $DEFAULT_LOG_DIR)
  --coordinator-bin PATH      Path to zisk-coordinator binary (env: ZISK_COORDINATOR_BIN, required)
  --port N                    Listening port                  (env: ZISK_COORDINATOR_PORT, default: $DEFAULT_PORT)
  --extra-args ARGS           Extra arguments for the binary  (env: ZISK_COORDINATOR_EXTRA_ARGS, optional)
EOF
if [[ "$OS" == "Darwin" ]]; then
  cat <<EOF
  --log-max-size MB           Max log file size in MB        (env: ZISK_COORDINATOR_LOG_MAX_SIZE_MB, default: $DEFAULT_LOG_MAX_SIZE_MB)
  --log-rotations N           Number of rotations kept       (env: ZISK_COORDINATOR_LOG_ROTATIONS, default: $DEFAULT_LOG_ROTATIONS)
EOF
fi
cat <<EOF
  -h, --help                  Show this help

EXAMPLES:
  # Basic install
  sudo ./$SCRIPT_NAME install \\
    --coordinator-bin /opt/zisk/bin/zisk-coordinator

  # Install with custom port and webhook URL
  sudo ./$SCRIPT_NAME install \\
    --coordinator-bin /opt/zisk/bin/zisk-coordinator \\
    --port 9090

EOF
  exit 0
}

# =============================================================================
# Parse CLI flags (override env vars)
# =============================================================================
COMMAND=""
if [[ $# -gt 0 && ( "$1" == "install" || "$1" == "uninstall" ) ]]; then
  COMMAND="$1"
  shift
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --coordinator-group) COORDINATOR_GROUP="$2"; shift 2 ;;
    --coordinator-user)  COORDINATOR_USER="$2";  shift 2 ;;
    --data-dir)          DATA_DIR="$2";          shift 2 ;;
    --log-dir)           LOG_DIR="$2";           shift 2 ;;
    --coordinator-bin)   COORDINATOR_BIN="$2";   shift 2 ;;
    --port)              PORT="$2";              shift 2 ;;
    --extra-args)        EXTRA_ARGS="$2";        shift 2 ;;
    --log-max-size)      LOG_MAX_SIZE_MB="$2";   shift 2 ;;
    --log-rotations)     LOG_ROTATIONS="$2";     shift 2 ;;
    -h|--help)           usage_install ;;
    install|uninstall)   COMMAND="$1"; shift ;;
    *) die "Unknown option: $1. Use --help for usage." ;;
  esac
done

# =============================================================================
# Validate required arguments
# =============================================================================
validate_args() {
  [[ -z "$COORDINATOR_BIN" ]] && { echo "[ERROR] --coordinator-bin (or ZISK_COORDINATOR_BIN) is required." >&2; usage_install; }
}

# =============================================================================
# Step: build ProgramArguments array for launchd plist (macOS)
# =============================================================================
build_program_args_plist() {
  local args=("${DATA_DIR}/${COORDINATOR_BIN_NAME}"
              -p "$PORT")

  # Simple word-split for extra args (avoid complex quoting in plist)
  if [[ -n "$EXTRA_ARGS" ]]; then
    read -ra extra_arr <<< "$EXTRA_ARGS"
    args+=("${extra_arr[@]}")
  fi

  printf "    <array>\n"
  for arg in "${args[@]}"; do
    printf "        <string>%s</string>\n" "$arg"
  done
  printf "    </array>\n"
}

# =============================================================================
# Step: build ExecStart line(s) for the systemd unit (Linux)
# =============================================================================
build_exec_start() {
  printf "ExecStart=%s \\\n" "${DATA_DIR}/${COORDINATOR_BIN_NAME}"
  printf "    -p %s \\\n" "$PORT"
  printf "    --no-save-proofs \\\n"
  printf "    --compressed-proofs"
  if [[ -n "$EXTRA_ARGS" ]]; then
    printf " \\\n    %s" "$EXTRA_ARGS"
  fi
  printf "\n"
}

# =============================================================================
# Main
# =============================================================================
[[ "$(id -u)" -ne 0 ]] && die "This script must be run as root (use sudo)."

[[ -z "$COMMAND" ]] && usage_short

case "$COMMAND" in
  install)
    validate_args
    utils_create_group "$COORDINATOR_GROUP"
    utils_create_user "$COORDINATOR_USER" "$COORDINATOR_GROUP" "Zisk Coordinator"
    utils_create_directories "$COORDINATOR_USER" "$COORDINATOR_GROUP" "$LOG_DIR" "$DATA_DIR"
    utils_install_binary "$COORDINATOR_BIN" "${DATA_DIR}/${COORDINATOR_BIN_NAME}"
    if [[ "$OS" == "Darwin" ]]; then
      utils_deploy_launchd_plist "$COORDINATOR_BIN_NAME" "com.zisk.coordinator" "$COORDINATOR_USER" "$COORDINATOR_GROUP" "$DATA_DIR" "$LOG_DIR" "$(build_program_args_plist)" "-10"
      utils_deploy_newsyslog "$COORDINATOR_BIN_NAME" "${LOG_DIR}/${COORDINATOR_BIN_NAME}.log" "$COORDINATOR_USER" "$COORDINATOR_GROUP" "$LOG_MAX_SIZE_MB" "$LOG_ROTATIONS"
      utils_load_launchd_service "com.zisk.coordinator" "/Library/LaunchDaemons/com.zisk.coordinator.plist"
    else
      utils_deploy_systemd_unit "$COORDINATOR_BIN_NAME" "Zisk Coordinator" "$COORDINATOR_USER" "$COORDINATOR_GROUP" "$DATA_DIR" "$(build_exec_start)" "3" ""
      utils_enable_and_start_systemd "$COORDINATOR_BIN_NAME"
    fi
    utils_print_post_install "$COORDINATOR_BIN_NAME" "com.zisk.coordinator" "$LOG_DIR"
    ;;
  uninstall)
    utils_uninstall "$COORDINATOR_BIN_NAME" "com.zisk.coordinator"
    ;;
esac
