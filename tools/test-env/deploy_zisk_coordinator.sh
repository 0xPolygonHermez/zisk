#!/usr/bin/env bash
# =============================================================================
# deploy_zisk_coordinator.sh
#
# Provides deploy_coordinator() to install and start zisk-coordinator service.
# =============================================================================
set -euo pipefail

OS="$(uname -s)"
COORDINATOR_BIN_NAME="zisk-coordinator"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/deploy_utils.sh"

# =============================================================================
# Defaults (internal)
# =============================================================================
COORD_DEFAULT_DATA_DIR="/var/lib/${COORDINATOR_BIN_NAME}"
COORD_DEFAULT_LOG_DIR="/var/log/${COORDINATOR_BIN_NAME}"
COORD_DEFAULT_LOG_MAX_SIZE_MB="100"
COORD_DEFAULT_LOG_ROTATIONS="5"

# =============================================================================
# Helpers
# =============================================================================
coordinator_build_program_args_plist() {
  local data_dir="$1"
  local api_port="$2"
  local cluster_port="$3"

  local args=("${data_dir}/${COORDINATOR_BIN_NAME}"
              --api-port "$api_port"
              --cluster-port "$cluster_port")

  printf "    <array>\n"
  for arg in "${args[@]}"; do
    printf "        <string>%s</string>\n" "$arg"
  done
  printf "    </array>\n"
}

coordinator_build_exec_start() {
  local data_dir="$1"
  local api_port="$2"
  local cluster_port="$3"

  printf "ExecStart=%s --api-port %s --cluster-port %s\n" "${data_dir}/${COORDINATOR_BIN_NAME}" "$api_port" "$cluster_port"
}

# =============================================================================
# deploy_coordinator GROUP USER API_PORT CLUSTER_PORT COORDINATOR_BIN
# =============================================================================
deploy_coordinator() {
  local coordinator_group="$1"
  local coordinator_user="$2"
  local api_port="$3"
  local cluster_port="$4"
  local coordinator_bin="$5"

  local data_dir="$COORD_DEFAULT_DATA_DIR"
  local log_dir="$COORD_DEFAULT_LOG_DIR"
  local log_max_size_mb="$COORD_DEFAULT_LOG_MAX_SIZE_MB"
  local log_rotations="$COORD_DEFAULT_LOG_ROTATIONS"

  [[ -z "$coordinator_bin" ]] && die "deploy_coordinator: coordinator_bin is required."

  utils_create_group "$coordinator_group"
  utils_create_user "$coordinator_user" "$coordinator_group" "Zisk Coordinator"
  utils_create_directories "$coordinator_user" "$coordinator_group" "$log_dir" "$data_dir"
  utils_install_binary "$coordinator_bin" "${data_dir}/${COORDINATOR_BIN_NAME}"

  if [[ "$OS" == "Darwin" ]]; then
    utils_deploy_launchd_plist "$COORDINATOR_BIN_NAME" "com.zisk.coordinator" "$coordinator_user" "$coordinator_group" "$data_dir" "$log_dir" "$(coordinator_build_program_args_plist "$data_dir" "$api_port" "$cluster_port")" "-10"
    utils_deploy_newsyslog "$COORDINATOR_BIN_NAME" "${log_dir}/${COORDINATOR_BIN_NAME}.log" "$coordinator_user" "$coordinator_group" "$log_max_size_mb" "$log_rotations"
    utils_load_launchd_service "com.zisk.coordinator" "/Library/LaunchDaemons/com.zisk.coordinator.plist"
  else
    utils_deploy_systemd_unit "$COORDINATOR_BIN_NAME" "Zisk Coordinator" "$coordinator_user" "$coordinator_group" "$data_dir" "$(coordinator_build_exec_start "$data_dir" "$api_port" "$cluster_port")" "3" ""
    utils_enable_and_start_systemd "$COORDINATOR_BIN_NAME"
  fi
}
