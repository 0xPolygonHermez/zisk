#!/usr/bin/env bash
# =============================================================================
# deploy_zisk_worker.sh
#
# Provides deploy_worker() to install and start zisk-worker service.
# =============================================================================
set -euo pipefail

OS="$(uname -s)"
WORKER_BIN_NAME="zisk-worker"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "${SCRIPT_DIR}/deploy_utils.sh"

# =============================================================================
# Defaults (internal)
# =============================================================================
DEFAULT_DATA_DIR="/var/lib/${WORKER_BIN_NAME}"
DEFAULT_LOG_DIR="/var/log/${WORKER_BIN_NAME}"
DEFAULT_LOG_MAX_SIZE_MB="100"
DEFAULT_LOG_ROTATIONS="5"
DEFAULT_WORKER_BIN="${HOME}/.zisk/bin/${WORKER_BIN_NAME}"
DEFAULT_WORKER_ID="worker-01"
DEFAULT_NO_MPI="true"
DEFAULT_HINTS_ENABLED="false"
DEFAULT_GPU_ENABLED="false"
DEFAULT_PROVINGKEY_DIR=""
DEFAULT_EXTRA_ARGS=""

# =============================================================================
# Helpers
# =============================================================================
resolve_mpi_params() {
  local no_mpi="$1"

  if [[ "$no_mpi" == "true" ]]; then
    echo "" "" ""
    return 0
  fi

  mpi_params
  echo "$MPI_NP" "$MPI_PPR" "$MPI_RAYON_NUM_THREADS"
}

build_program_args_plist() {
  local data_dir="$1"
  local coordinator_url="$2"
  local no_mpi="$3"
  local mpi_processes="$4"
  local mpi_ppr_numa="$5"
  local mpi_threads="$6"
  local provingkey_dir="$7"
  local worker_id="$8"
  local hints_enabled="$9"
  local gpu_enabled="${10}"
  local extra_args="${11}"

  local args=()
  if [[ "$no_mpi" == "true" ]]; then
    args+=("${data_dir}/${WORKER_BIN_NAME}")
  else
    args+=(mpirun --report-bindings --allow-run-as-root
           -np "$mpi_processes"
           -map-by "ppr:${mpi_ppr_numa}:numa" --bind-to numa --rank-by slot
           -x "RAYON_NUM_THREADS=${mpi_threads}"
           "${data_dir}/${WORKER_BIN_NAME}")
  fi

  args+=(--coordinator-url "${coordinator_url}"
         --worker-id "${worker_id}")
  [[ -n "$provingkey_dir" ]] && args+=(-k "$provingkey_dir")
  [[ "$hints_enabled" == "true" ]] && args+=(--hints)
  [[ "$gpu_enabled" == "true" ]] && args+=(--gpu)

  if [[ -n "$extra_args" ]]; then
    read -ra extra_arr <<< "$extra_args"
    args+=("${extra_arr[@]}")
  fi

  printf "    <array>\n"
  for arg in "${args[@]}"; do
    printf "        <string>%s</string>\n" "$arg"
  done
  printf "    </array>\n"
}

build_exec_start() {
  local data_dir="$1"
  local coordinator_url="$2"
  local no_mpi="$3"
  local mpi_processes="$4"
  local mpi_ppr_numa="$5"
  local mpi_threads="$6"
  local provingkey_dir="$7"
  local worker_id="$8"
  local hints_enabled="$9"
  local gpu_enabled="${10}"
  local extra_args="${11}"

  local hints_arg=""
  local gpu_arg=""
  [[ "$hints_enabled" == "true" ]] && hints_arg=" --hints"
  [[ "$gpu_enabled" == "true" ]] && gpu_arg=" --gpu"

  local common_args="--coordinator-url ${coordinator_url}"
  [[ -n "$provingkey_dir" ]] && common_args+=" -k ${provingkey_dir}"
  common_args+=" --worker-id ${worker_id}${hints_arg}${gpu_arg}"

  if [[ -n "$extra_args" ]]; then
    common_args+=" ${extra_args}"
  fi

  if [[ "$no_mpi" == "true" ]]; then
    printf "ExecStart=%s %s\n" "${data_dir}/${WORKER_BIN_NAME}" "$common_args"
  else
    printf "ExecStart=mpirun --report-bindings --allow-run-as-root -np %s -map-by ppr:%s:numa --bind-to numa --rank-by slot -x RAYON_NUM_THREADS=%s %s %s\n" "$mpi_processes" "$mpi_ppr_numa" "$mpi_threads" "${data_dir}/${WORKER_BIN_NAME}" "$common_args"
  fi
}

# =============================================================================
# deploy_worker WORKER_GROUP WORKER_USER COORDINATOR_URL
# =============================================================================
deploy_worker() {
  local worker_group="$1"
  local worker_user="$2"
  local coordinator_url="$3"

  local data_dir="$DEFAULT_DATA_DIR"
  local log_dir="$DEFAULT_LOG_DIR"
  local log_max_size_mb="$DEFAULT_LOG_MAX_SIZE_MB"
  local log_rotations="$DEFAULT_LOG_ROTATIONS"
  local worker_bin="$DEFAULT_WORKER_BIN"
  local worker_id="$DEFAULT_WORKER_ID"
  local no_mpi="$DEFAULT_NO_MPI"
  local hints_enabled="$DEFAULT_HINTS_ENABLED"
  local gpu_enabled="$DEFAULT_GPU_ENABLED"
  local provingkey_dir="$DEFAULT_PROVINGKEY_DIR"
  local extra_args="$DEFAULT_EXTRA_ARGS"
  local mpi_processes=""
  local mpi_ppr_numa=""
  local mpi_threads=""

  [[ -z "$coordinator_url" ]] && die "deploy_worker: coordinator_url is required."
  [[ -z "$worker_bin" ]] && die "deploy_worker: worker binary path is empty."

  read -r mpi_processes mpi_ppr_numa mpi_threads <<< "$(resolve_mpi_params "$no_mpi")"

  utils_create_group "$worker_group"
  utils_create_user "$worker_user" "$worker_group" "Zisk Worker"
  utils_create_directories "$worker_user" "$worker_group" "$log_dir" "$data_dir"
  utils_install_binary "$worker_bin" "${data_dir}/${WORKER_BIN_NAME}"

  if [[ "$OS" == "Darwin" ]]; then
    utils_deploy_launchd_plist "$WORKER_BIN_NAME" "com.zisk.worker" "$worker_user" "$worker_group" "$data_dir" "$log_dir" "$(build_program_args_plist "$data_dir" "$coordinator_url" "$no_mpi" "$mpi_processes" "$mpi_ppr_numa" "$mpi_threads" "$provingkey_dir" "$worker_id" "$hints_enabled" "$gpu_enabled" "$extra_args")" "-10"
    utils_deploy_newsyslog "$WORKER_BIN_NAME" "${log_dir}/${WORKER_BIN_NAME}.log" "$worker_user" "$worker_group" "$log_max_size_mb" "$log_rotations"
    utils_load_launchd_service "com.zisk.worker" "/Library/LaunchDaemons/com.zisk.worker.plist"
  else
    utils_deploy_systemd_unit "$WORKER_BIN_NAME" "Zisk Worker" "$worker_user" "$worker_group" "$data_dir" "$(build_exec_start "$data_dir" "$coordinator_url" "$no_mpi" "$mpi_processes" "$mpi_ppr_numa" "$mpi_threads" "$provingkey_dir" "$worker_id" "$hints_enabled" "$gpu_enabled" "$extra_args")" "5" "-10"
    utils_enable_and_start_systemd "$WORKER_BIN_NAME"
  fi
}
