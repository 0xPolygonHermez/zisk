#!/usr/bin/env bash
# =============================================================================
# deploy_zisk_worker.sh
#
# Deploys the zisk-worker service (install/uninstall).
#
# Variables can be set via CLI flags or environment variables.
# CLI flags take precedence over environment variables.
# =============================================================================
set -euo pipefail

OS="$(uname -s)"
SCRIPT_NAME="$(basename "$0")"
WORKER_BIN_NAME="zisk-worker"

source "$(dirname "$0")/deploy_utils.sh"

# =============================================================================
# Defaults
# =============================================================================
DEFAULT_WORKER_GROUP="zisk"
DEFAULT_WORKER_USER="zisk"
DEFAULT_DATA_DIR="/var/lib/${WORKER_BIN_NAME}"
DEFAULT_COORDINATOR_URL="http://localhost:8080"
DEFAULT_NO_MPI="false"
DEFAULT_COMPUTE_CAPACITY="10"
DEFAULT_HINTS_ENABLED="false"
# macOS-only log settings
DEFAULT_LOG_DIR="/var/log/${WORKER_BIN_NAME}"
DEFAULT_LOG_MAX_SIZE_MB="100"
DEFAULT_LOG_ROTATIONS="5"

# =============================================================================
# Resolve values: env vars override defaults, CLI flags override env vars
# =============================================================================
WORKER_GROUP="${ZISK_WORKER_GROUP:-$DEFAULT_WORKER_GROUP}"
WORKER_USER="${ZISK_WORKER_USER:-$DEFAULT_WORKER_USER}"
DATA_DIR="${ZISK_WORKER_DATA_DIR:-$DEFAULT_DATA_DIR}"
INPUTS_FOLDER="${ZISK_WORKER_INPUTS_FOLDER:-}"
WORKER_BIN="${ZISK_WORKER_BIN:-}"
COORDINATOR_URL="${ZISK_WORKER_COORDINATOR_URL:-$DEFAULT_COORDINATOR_URL}"
NO_MPI="${ZISK_WORKER_NO_MPI:-$DEFAULT_NO_MPI}"
MPI_PROCESSES="${ZISK_WORKER_MPI_PROCESSES:-}"
MPI_PPR_NUMA="${ZISK_WORKER_MPI_PPR_NUMA:-}"
MPI_THREADS="${ZISK_WORKER_MPI_THREADS:-}"
PROVINGKEY_DIR="${ZISK_PROVINGKEY_DIR:-}"
WORKER_ID="${ZISK_WORKER_ID:-}"
COMPUTE_CAPACITY="${ZISK_WORKER_COMPUTE_CAPACITY:-$DEFAULT_COMPUTE_CAPACITY}"
HINTS_ENABLED="${ZISK_HINTS_ENABLED:-$DEFAULT_HINTS_ENABLED}"
EXTRA_ARGS="${ZISK_WORKER_EXTRA_ARGS:-}"
# macOS-only log settings (ignored on Linux)
LOG_DIR="${ZISK_WORKER_LOG_DIR:-$DEFAULT_LOG_DIR}"
LOG_MAX_SIZE_MB="${ZISK_WORKER_LOG_MAX_SIZE_MB:-$DEFAULT_LOG_MAX_SIZE_MB}"
LOG_ROTATIONS="${ZISK_WORKER_LOG_ROTATIONS:-$DEFAULT_LOG_ROTATIONS}"

# =============================================================================
# Helpers
# =============================================================================
usage_short() {
  cat <<EOF
Usage: $SCRIPT_NAME <command> [OPTIONS]

COMMANDS:
  install     Install and start the ${WORKER_BIN_NAME} service
  uninstall   Stop and remove the ${WORKER_BIN_NAME} service

Run '$SCRIPT_NAME install --help' for install options.

EXAMPLES:
  sudo ./$SCRIPT_NAME install --worker-bin /opt/zisk/bin/zisk-worker --worker-id worker-01 --coordinator-url http://192.168.1.10:8080
  sudo ./$SCRIPT_NAME uninstall
EOF
  exit 0
}

usage_install() {
  cat <<EOF
Usage: $SCRIPT_NAME install [OPTIONS]

Install the ${WORKER_BIN_NAME} service.

OPTIONS:
  --worker-group GROUP        System group name              (env: ZISK_WORKER_GROUP, default: $DEFAULT_WORKER_GROUP)
  --worker-user USER          System user name               (env: ZISK_WORKER_USER, default: $DEFAULT_WORKER_USER)
  --data-dir DIR              Data directory                 (env: ZISK_WORKER_DATA_DIR, default: $DEFAULT_DATA_DIR)
  --inputs-folder DIR         Inputs folder                  (env: ZISK_WORKER_INPUTS_FOLDER, default: DATA_DIR/inputs)
  --worker-bin PATH           Path to zisk-worker binary     (env: ZISK_WORKER_BIN, required)
  --coordinator-url URL       Coordinator URL                (env: ZISK_WORKER_COORDINATOR_URL, default: $DEFAULT_COORDINATOR_URL)
  --no-mpi                    Disable MPI mode               (env: ZISK_WORKER_NO_MPI)
  --mpi-processes N           Number of MPI processes        (env: ZISK_WORKER_MPI_PROCESSES, auto-detected unless --no-mpi)
  --mpi-numa-ppr N            Processes per NUMA node        (env: ZISK_WORKER_MPI_PPR_NUMA, auto-detected unless --no-mpi)
  --mpi-threads N             Threads per MPI process        (env: ZISK_WORKER_MPI_THREADS, auto-detected unless --no-mpi)
  --provingkey-dir DIR        Proving key directory          (env: ZISK_PROVINGKEY_DIR, optional)
  --worker-id ID              Worker identifier              (env: ZISK_WORKER_ID, required)
  --compute-capacity N        Compute capacity               (env: ZISK_WORKER_COMPUTE_CAPACITY, default: $DEFAULT_COMPUTE_CAPACITY)
  --hints                     Enable hints flag              (env: ZISK_HINTS_ENABLED)
  --extra-args ARGS           Extra arguments for the binary (env: ZISK_WORKER_EXTRA_ARGS, optional)
EOF
if [[ "$OS" == "Darwin" ]]; then
  cat <<EOF
  --log-dir DIR               Log directory                  (env: ZISK_WORKER_LOG_DIR,  default: $DEFAULT_LOG_DIR)
  --log-max-size MB           Max log file size in MB        (env: ZISK_WORKER_LOG_MAX_SIZE_MB, default: $DEFAULT_LOG_MAX_SIZE_MB)
  --log-rotations N           Number of rotations kept       (env: ZISK_WORKER_LOG_ROTATIONS, default: $DEFAULT_LOG_ROTATIONS)
EOF
fi
cat <<EOF
  -h, --help                  Show this help

EXAMPLES:
  # Install without MPI
  sudo ./$SCRIPT_NAME install \\
    --worker-bin /opt/zisk/bin/zisk-worker \\
    --coordinator-url http://192.168.1.10:8080 \\
    --worker-id worker-01 \\
    --no-mpi

  # Install with custom MPI parameters
  sudo ./$SCRIPT_NAME install \\
    --worker-bin /opt/zisk/zisk-worker \\
    --coordinator-url http://192.168.1.10:8080 \\
    --worker-id worker-01 \\
    --mpi-processes 4 \\
    --mpi-threads 8

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
    --worker-group)      WORKER_GROUP="$2";      shift 2 ;;
    --worker-user)       WORKER_USER="$2";       shift 2 ;;
    --data-dir)          DATA_DIR="$2";          shift 2 ;;
    --inputs-folder)     INPUTS_FOLDER="$2";     shift 2 ;;
    --worker-bin)        WORKER_BIN="$2";        shift 2 ;;
    --coordinator-url)   COORDINATOR_URL="$2";   shift 2 ;;
    --no-mpi)            NO_MPI="true";          shift   ;;
    --mpi-processes)     MPI_PROCESSES="$2";     shift 2 ;;
    --mpi-numa-ppr)      MPI_PPR_NUMA="$2";      shift 2 ;;
    --mpi-threads)       MPI_THREADS="$2";       shift 2 ;;
    --provingkey-dir)    PROVINGKEY_DIR="$2";    shift 2 ;;
    --worker-id)         WORKER_ID="$2";         shift 2 ;;
    --compute-capacity)  COMPUTE_CAPACITY="$2";  shift 2 ;;
    --hints)             HINTS_ENABLED="true";   shift   ;;
    --extra-args)        EXTRA_ARGS="$2";        shift 2 ;;
    --log-dir)           LOG_DIR="$2";           shift 2 ;;
    --log-max-size)      LOG_MAX_SIZE_MB="$2";   shift 2 ;;
    --log-rotations)     LOG_ROTATIONS="$2";     shift 2 ;;
    -h|--help)           usage_install ;;
    install|uninstall)   COMMAND="$1"; shift ;;
    *) die "Unknown option: $1. Use --help for usage." ;;
  esac
done

# Derived values
[[ -z "$INPUTS_FOLDER" ]] && INPUTS_FOLDER="${DATA_DIR}/inputs"

# =============================================================================
# Validate required arguments and resolve MPI parameters
# =============================================================================
validate_args() {
  [[ -z "$WORKER_BIN" ]] && { echo "[ERROR] --worker-bin (or ZISK_WORKER_BIN) is required." >&2; usage_install; }
  [[ -z "$WORKER_ID" ]] && { echo "[ERROR] --worker-id (or ZISK_WORKER_ID) is required." >&2; usage_install; }

  # Enforce that MPI flags are only used without --no-mpi
  if [[ "$NO_MPI" == "true" ]]; then
    [[ -n "$MPI_PROCESSES" ]] && die "--mpi-processes cannot be used with --no-mpi."
    [[ -n "$MPI_PPR_NUMA" ]]  && die "--mpi-numa-ppr cannot be used with --no-mpi."
    [[ -n "$MPI_THREADS" ]]   && die "--mpi-threads cannot be used with --no-mpi."
  else
    # Enforce that MPI_PROCESSES, MPI_PPR_NUMA and MPI_THREADS are all set or all unset
    local mpi_set=0
    [[ -n "$MPI_PROCESSES" ]] && (( mpi_set++ ))
    [[ -n "$MPI_PPR_NUMA" ]]  && (( mpi_set++ ))
    [[ -n "$MPI_THREADS" ]]   && (( mpi_set++ ))
    if [[ "$mpi_set" -gt 0 && "$mpi_set" -lt 3 ]]; then
      die "--mpi-processes, --mpi-numa-ppr and --mpi-threads must all be specified together."
    fi
    # Auto-detect MPI parameters if none were provided
    if [[ "$mpi_set" -eq 0 ]]; then
      info "MPI parameters not specified, detecting from hardware..."
      mpi_params
      MPI_PROCESSES="$MPI_NP"
      MPI_PPR_NUMA="$MPI_PPR"
      MPI_THREADS="$MPI_RAYON_NUM_THREADS"
      info "Auto-detected: MPI_PROCESSES=${MPI_PROCESSES}, MPI_PPR_NUMA=${MPI_PPR_NUMA}, MPI_THREADS=${MPI_THREADS}"
    fi
  fi
}

# =============================================================================
# Step: build ProgramArguments array for launchd plist (macOS)
# =============================================================================
build_program_args_plist() {
  local args=()
  if [[ "$NO_MPI" == "true" ]]; then
    args+=("${DATA_DIR}/${WORKER_BIN_NAME}")
  else
    args+=(mpirun --report-bindings --allow-run-as-root
           -np "$MPI_PROCESSES"
           -map-by "ppr:${MPI_PPR_NUMA}:numa" --bind-to numa --rank-by slot
           -x "RAYON_NUM_THREADS=${MPI_THREADS}"
           "${DATA_DIR}/${WORKER_BIN_NAME}")
  fi
  args+=(-e "${DATA_DIR}/zec-reth.elf"
         --coordinator-url "${COORDINATOR_URL}")
  [[ -n "$PROVINGKEY_DIR" ]] && args+=(-k "${PROVINGKEY_DIR}")
  args+=(--inputs-folder "${INPUTS_FOLDER}"
         --worker-id "${WORKER_ID}"
         --compute-capacity "${COMPUTE_CAPACITY}")
  [[ "$HINTS_ENABLED" == "true" ]] && args+=(--hints)

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
# Step: build ExecStart line(s) for the service unit
# =============================================================================
build_exec_start() {
  local hints_arg=""
  [[ "$HINTS_ENABLED" == "true" ]] && hints_arg=" --hints"

  local common_args="\
    -e ${DATA_DIR}/zec-reth.elf \\
    --coordinator-url \"${COORDINATOR_URL}\""
  [[ -n "$PROVINGKEY_DIR" ]] && common_args+=" \\
    -k ${PROVINGKEY_DIR}"
  common_args+=" \\
    --inputs-folder ${INPUTS_FOLDER} \\
    --worker-id \"${WORKER_ID}\" \\
    --compute-capacity ${COMPUTE_CAPACITY}${hints_arg}"

  if [[ -n "$EXTRA_ARGS" ]]; then
    common_args+=" \\
    ${EXTRA_ARGS}"
  fi

  if [[ "$NO_MPI" == "true" ]]; then
    printf "ExecStart=%s \\\n" "${DATA_DIR}/${WORKER_BIN_NAME}"
    printf "%s\n" "$common_args"
  else
    printf "ExecStart=mpirun \\\n"
    printf "    --report-bindings \\\n"
    printf "    --allow-run-as-root \\\n"
    printf "    -np %s \\\n" "$MPI_PROCESSES"
    printf "    -map-by ppr:%s:numa \\\n" "$MPI_PPR_NUMA"
    printf "    --bind-to numa \\\n"
    printf "    --rank-by slot \\\n"
    printf "    -x RAYON_NUM_THREADS=%s \\\n" "$MPI_THREADS"
    printf "    %s \\\n" "${DATA_DIR}/${WORKER_BIN_NAME}"
    printf "%s\n" "$common_args"
  fi
}

# =============================================================================
# Main
# =============================================================================
[[ "$(id -u)" -ne 0 ]] && die "This script must be run as root (use sudo)."

[[ -z "$COMMAND" ]] && usage_short

case "$COMMAND" in
  install)
    validate_args
    utils_create_group "$WORKER_GROUP"
    utils_create_user "$WORKER_USER" "$WORKER_GROUP" "Zisk Worker"
    utils_create_directories "$WORKER_USER" "$WORKER_GROUP" "$LOG_DIR" "$DATA_DIR" "$INPUTS_FOLDER"
    utils_install_binary "$WORKER_BIN" "${DATA_DIR}/${WORKER_BIN_NAME}"
    if [[ "$OS" == "Darwin" ]]; then
      utils_deploy_launchd_plist "$WORKER_BIN_NAME" "com.zisk.worker" "$WORKER_USER" "$WORKER_GROUP" "$DATA_DIR" "$LOG_DIR" "$(build_program_args_plist)" "-10"
      utils_deploy_newsyslog "$WORKER_BIN_NAME" "${LOG_DIR}/${WORKER_BIN_NAME}.log" "$WORKER_USER" "$WORKER_GROUP" "$LOG_MAX_SIZE_MB" "$LOG_ROTATIONS"
      utils_load_launchd_service "com.zisk.worker" "/Library/LaunchDaemons/com.zisk.worker.plist"
    else
      utils_deploy_systemd_unit "$WORKER_BIN_NAME" "Zisk Worker" "$WORKER_USER" "$WORKER_GROUP" "$DATA_DIR" "$(build_exec_start)" "5" "-10"
      utils_enable_and_start_systemd "$WORKER_BIN_NAME"
    fi
    utils_print_post_install "$WORKER_BIN_NAME" "com.zisk.worker" "$LOG_DIR"
    ;;
  uninstall)
    utils_uninstall "$WORKER_BIN_NAME" "com.zisk.worker"
    ;;
esac
