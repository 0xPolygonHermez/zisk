#!/usr/bin/env bash
# =============================================================================
# deploy_utils.sh
#
# Shared utility functions for deploy zisk services scripts.
# Source this file: source "$(dirname "$0")/deploy_utils.sh"
# =============================================================================

OS="$(uname -s)"

# =============================================================================
# Logging helpers
# =============================================================================
info() { echo "[INFO]  $*"; }
die()  { echo "[ERROR] $*" >&2; exit 1; }

# =============================================================================
# utils_load_env_file [ARGS...]
# Loads environment variables from .env file.
# First pass: checks if --env flag was provided to override default .env location.
# If --env not provided, checks if ./.env exists and loads it.
# Usage: utils_load_env_file "$@"  (pass all script arguments)
# =============================================================================
utils_load_env_file() {
  local env_file="./.env"
  local prev_arg=""

  # First pass: check if --env flag was provided
  for arg in "$@"; do
    if [[ "$prev_arg" == "--env" ]]; then
      env_file="$arg"
      break
    fi
    prev_arg="$arg"
  done

  # Load .env file if it exists
  if [[ -f "$env_file" ]]; then
    info "Loading environment variables from $env_file"
    source "$env_file"
  fi
}

# =============================================================================
# utils_create_group GROUP
# Creates a system group if it does not already exist.
# =============================================================================
utils_create_group() {
  local group="$1"
  info "Ensuring group '${group}' exists..."
  if [[ "$OS" == "Darwin" ]]; then
    if ! dscl . -read "/Groups/${group}" &>/dev/null; then
      local gid
      gid=$(( $(dscl . -list /Groups PrimaryGroupID | awk '{print $2}' | sort -n | tail -1) + 1 ))
      sudo dscl . -create "/Groups/${group}"
      sudo dscl . -create "/Groups/${group}" PrimaryGroupID "$gid"
      sudo dscl . -create "/Groups/${group}" RecordName      "$group"
    else
      info "Group '${group}' already exists, skipping."
    fi
  else
    if ! getent group "$group" &>/dev/null; then
      sudo groupadd --system "$group"
    else
      info "Group '${group}' already exists, skipping."
    fi
  fi
}

# =============================================================================
# utils_create_user USER GROUP REALNAME
# Creates a system user (no login, no home) if it does not already exist.
# =============================================================================
utils_create_user() {
  local user="$1" group="$2" realname="$3"
  info "Ensuring user '${user}' exists..."
  if [[ "$OS" == "Darwin" ]]; then
    if ! dscl . -read "/Users/${user}" &>/dev/null; then
      local uid gid
      uid=$(( $(dscl . -list /Users UniqueID | awk '{print $2}' | sort -n | tail -1) + 1 ))
      gid=$(dscl . -read "/Groups/${group}" PrimaryGroupID | awk '{print $2}')
      sudo dscl . -create "/Users/${user}"
      sudo dscl . -create "/Users/${user}" UniqueID         "$uid"
      sudo dscl . -create "/Users/${user}" PrimaryGroupID   "$gid"
      sudo dscl . -create "/Users/${user}" UserShell        /usr/bin/false
      sudo dscl . -create "/Users/${user}" RealName         "$realname"
      sudo dscl . -create "/Users/${user}" NFSHomeDirectory /var/empty
    else
      info "User '${user}' already exists, skipping."
    fi
  else
    if ! id "$user" &>/dev/null; then
      sudo useradd \
        --system \
        --gid "$group" \
        --no-create-home \
        --shell /usr/sbin/nologin \
        "$user"
    else
      info "User '${user}' already exists, skipping."
    fi
  fi
}

# =============================================================================
# utils_create_directories USER GROUP LOG_DIR DIR [DIR ...]
# Creates one or more directories owned by USER:GROUP with mode 0755.
# On macOS, also creates LOG_DIR (pass "" to skip).
# =============================================================================
utils_create_directories() {
  local user="$1" group="$2" log_dir="$3"
  shift 3
  info "Creating directories..."
  local dirs=("$@")
  if [[ "$OS" == "Darwin" && -n "$log_dir" ]]; then
    dirs+=("$log_dir")
  fi
  for dir in "${dirs[@]}"; do
    sudo mkdir -p "$dir"
    sudo chown "${user}:${group}" "$dir"
    sudo chmod 0755 "$dir"
  done
}

# =============================================================================
# utils_install_binary SRC DEST
# Copies a binary to DEST with root ownership and 0755 permissions.
#   macOS — root:wheel 0755
#   Linux — root:root  0755
# =============================================================================
utils_install_binary() {
  local src="$1" dest="$2"
  info "Installing binary to '${dest}'..."
  if [[ "$OS" == "Darwin" ]]; then
    sudo install -m 0755 -o root -g wheel "$src" "$dest"
  else
    sudo install -m 0755 -o root -g root "$src" "$dest"
  fi
  info "Binary installed at ${dest}."
}

# =============================================================================
# utils_deploy_newsyslog SERVICE LOG_FILE USER GROUP MAX_MB ROTATIONS
# Deploys a newsyslog rotation config for a single log file (macOS only).
# =============================================================================
utils_deploy_newsyslog() {
  local service="$1" log_file="$2" user="$3" group="$4" max_mb="$5" rotations="$6"
  local conf="/etc/newsyslog.d/${service}.conf"
  info "Deploying newsyslog rotation config at ${conf}..."
  # Format: logfile [owner:group] mode count size(KB) when flags [/pidfile] [signum]
  sudo tee "$conf" > /dev/null <<NEWSYSLOG
# ${service} log rotation — max ${max_mb}MB per file, keep ${rotations} rotations, compressed
${log_file}  ${user}:${group}  640  ${rotations}  $(( max_mb * 1024 ))  *  JG
NEWSYSLOG
  sudo chmod 0644 "$conf"
}

# =============================================================================
# utils_deploy_launchd_plist BIN_NAME LAUNCHD_LABEL USER GROUP DATA_DIR LOG_DIR PROGRAM_ARGS [NICE]
# Writes a launchd daemon plist to /Library/LaunchDaemons/${LAUNCHD_LABEL}.plist.
# PROGRAM_ARGS: the <array>...</array> block from build_program_args_plist().
# NICE: optional CPU scheduling priority integer (e.g. -10); omit or pass "" to skip.
# =============================================================================
utils_deploy_launchd_plist() {
  local bin_name="$1" launchd_label="$2" svc_user="$3" svc_group="$4"
  local data_dir="$5" log_dir="$6" program_args="$7" nice="${8:-}"
  local plist="/Library/LaunchDaemons/${launchd_label}.plist"

  info "Deploying ${plist}..."

  local nice_block=""
  if [[ -n "$nice" ]]; then
    nice_block="    <key>Nice</key>
    <integer>${nice}</integer>
"
  fi

  sudo tee "$plist" > /dev/null <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${launchd_label}</string>

    <key>ProgramArguments</key>
${program_args}
    <key>UserName</key>
    <string>${svc_user}</string>

    <key>GroupName</key>
    <string>${svc_group}</string>

    <key>WorkingDirectory</key>
    <string>${data_dir}</string>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>${log_dir}/${bin_name}.log</string>

    <key>StandardErrorPath</key>
    <string>${log_dir}/${bin_name}.log</string>

    <key>ProcessType</key>
    <string>Interactive</string>

${nice_block}    <key>SoftResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>65535</integer>
    </dict>
</dict>
</plist>
<!-- ${bin_name}:DATA_DIR=${data_dir} -->
<!-- ${bin_name}:LOG_DIR=${log_dir} -->
<!-- ${bin_name}:SVC_USER=${svc_user} -->
<!-- ${bin_name}:SVC_GROUP=${svc_group} -->
PLIST

  sudo chown root:wheel "$plist"
  sudo chmod 0644 "$plist"
}

# =============================================================================
# utils_load_launchd_service LABEL PLIST
# Unloads (if loaded) and loads a launchd daemon plist.
# =============================================================================
utils_load_launchd_service() {
  local label="$1" plist="$2"
  info "Loading and starting ${label} via launchd..."
  sudo launchctl unload "$plist" 2>/dev/null || true
  sudo launchctl load -w "$plist"
}

# =============================================================================
# utils_deploy_systemd_unit BIN_NAME DESCRIPTION USER GROUP DATA_DIR EXEC_START RESTART_SEC [NICE]
# Writes a systemd service unit file to /etc/systemd/system/${BIN_NAME}.service.
# EXEC_START: the ExecStart= line(s) to embed verbatim.
# NICE: optional CPU scheduling priority (e.g. -10); omit or pass "" to skip.
# =============================================================================
utils_deploy_systemd_unit() {
  local bin_name="$1" description="$2" svc_user="$3" svc_group="$4"
  local data_dir="$5" exec_start="$6" restart_sec="$7" nice="${8:-}"

  local nice_line=""
  [[ -n "$nice" ]] && nice_line="Nice=${nice}"

  info "Deploying /etc/systemd/system/${bin_name}.service..."

  sudo tee "/etc/systemd/system/${bin_name}.service" > /dev/null <<UNIT
[Unit]
Description=${description}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${svc_user}
Group=${svc_group}
Restart=always
RestartSec=${restart_sec}
LimitNOFILE=65535
${nice_line}
WorkingDirectory=${data_dir}
${exec_start}
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target

# ${bin_name}:DATA_DIR=${data_dir}
# ${bin_name}:SVC_USER=${svc_user}
# ${bin_name}:SVC_GROUP=${svc_group}
UNIT
}

# =============================================================================
# utils_enable_and_start_systemd SERVICE
# Reloads systemd and enables + starts a service unit.
# =============================================================================
utils_enable_and_start_systemd() {
  local service="$1"
  info "Reloading systemd and enabling/starting ${service}..."
  sudo systemctl daemon-reload
  sudo systemctl enable "$service"
  sudo systemctl start  "$service"
}

# =============================================================================
# utils_print_post_install BIN_NAME LAUNCHD_LABEL LOG_DIR
# Prints service management hints after a successful install.
# =============================================================================
utils_print_post_install() {
  local bin_name="$1" launchd_label="$2" log_dir="$3"
  info "${bin_name} installed and running."
  if [[ "$OS" == "Darwin" ]]; then
    echo ""
    echo "${bin_name} service management (macOS):"
    echo "  Start:  sudo launchctl start ${launchd_label}"
    echo "  Stop:   sudo launchctl stop ${launchd_label}"
    echo "  Status: sudo launchctl print system/${launchd_label}"
    echo "  Logs:   tail -f ${log_dir}/${bin_name}.log"
  else
    echo ""
    echo "${bin_name} service management:"
    echo "  Start:  sudo systemctl start ${bin_name}"
    echo "  Stop:   sudo systemctl stop ${bin_name}"
    echo "  Status: sudo systemctl status ${bin_name}"
    echo "  Logs:   sudo journalctl -u ${bin_name} -f"
  fi
}

# =============================================================================
# utils_uninstall BIN_NAME LAUNCHD_LABEL
# Stops, removes, and optionally cleans up a service installed by deploy-zisk-*.sh.
# Metadata is read from the service config file written at install time.
# Metadata keys expected: BIN_NAME:DATA_DIR, BIN_NAME:LOG_DIR (macOS plist only),
#                         BIN_NAME:SVC_USER, BIN_NAME:SVC_GROUP
# =============================================================================
utils_uninstall() {
  local bin_name="$1" launchd_label="$2"
  local plist="/Library/LaunchDaemons/${launchd_label}.plist"
  local unit="/etc/systemd/system/${bin_name}.service"

  # Check service config exists before asking for confirmation
  if [[ "$OS" == "Darwin" ]]; then
    [[ ! -f "$plist" ]] && die "${bin_name} is not installed (${plist} not found)."
  else
    [[ ! -f "$unit" ]] && die "${bin_name} is not installed (${unit} not found)."
  fi

  utils_confirm_uninstall "$bin_name"

  info "Uninstalling ${bin_name}..."

  # Read install-time metadata from service config file
  local data_dir log_dir svc_user svc_group
  if [[ "$OS" == "Darwin" ]]; then
    data_dir=$(grep  "<!-- ${bin_name}:DATA_DIR=" "$plist" | sed "s/.*DATA_DIR=\(.*\) -->/\1/")
    log_dir=$(grep   "<!-- ${bin_name}:LOG_DIR="  "$plist" | sed "s/.*LOG_DIR=\(.*\) -->/\1/")
    svc_user=$(grep  "<!-- ${bin_name}:SVC_USER=" "$plist" | sed "s/.*SVC_USER=\(.*\) -->/\1/")
    svc_group=$(grep "<!-- ${bin_name}:SVC_GROUP=" "$plist" | sed "s/.*SVC_GROUP=\(.*\) -->/\1/")
  else
    data_dir=$(grep  "# ${bin_name}:DATA_DIR="  "$unit" | sed 's/.*DATA_DIR=\(.*\)/\1/')
    svc_user=$(grep  "# ${bin_name}:SVC_USER="  "$unit" | sed 's/.*SVC_USER=\(.*\)/\1/')
    svc_group=$(grep "# ${bin_name}:SVC_GROUP=" "$unit" | sed 's/.*SVC_GROUP=\(.*\)/\1/')
    log_dir=""
  fi

  # Stop and remove service
  if [[ "$OS" == "Darwin" ]]; then
    info "Stopping and unloading launchd service..."
    sudo launchctl unload "$plist" 2>/dev/null || true
    sudo rm -f "$plist"
    info "Removed ${plist}."
    local nsconf="/etc/newsyslog.d/${bin_name}.conf"
    [[ -f "$nsconf" ]] && sudo rm -f "$nsconf" && info "Removed ${nsconf}."
  else
    if systemctl list-unit-files "${bin_name}.service" &>/dev/null; then
      info "Stopping and disabling systemd service..."
      sudo systemctl stop    "$bin_name" 2>/dev/null || true
      sudo systemctl disable "$bin_name" 2>/dev/null || true
    fi
    sudo rm -f "$unit"
    sudo systemctl daemon-reload
    info "Removed ${unit}."
  fi

  utils_remove_user_and_group "$svc_user" "$svc_group"
  [[ -n "$log_dir" ]] && utils_remove_dir "$log_dir" "log directory"
  utils_remove_dir "$data_dir" "data directory"

  info "${bin_name} uninstalled."
}

# =============================================================================
# utils_confirm_uninstall SERVICE
# Prompts "Are you sure?" and exits 0 if the user says no.
# =============================================================================
utils_confirm_uninstall() {
  local service="$1"
  local confirm="n"
  read -r -p "Are you sure you want to uninstall ${service}? [y/N] " confirm
  [[ "$(echo "$confirm" | tr '[:upper:]' '[:lower:]')" != "y" ]] && { info "Uninstall cancelled."; exit 0; }
}

# =============================================================================
# utils_remove_user_and_group USER GROUP
# Interactively removes a system user and group.
# =============================================================================
utils_remove_user_and_group() {
  local user="$1" group="$2"
  local remove="n"
  read -r -p "Remove system user '${user}' and group '${group}'? [y/N] " remove
  if [[ "$(echo "$remove" | tr '[:upper:]' '[:lower:]')" == "y" ]]; then
    if [[ "$OS" == "Darwin" ]]; then
      dscl . -read "/Users/${user}" &>/dev/null && sudo dscl . -delete "/Users/${user}" && info "Removed user '${user}'."
      dscl . -read "/Groups/${group}" &>/dev/null && sudo dscl . -delete "/Groups/${group}" && info "Removed group '${group}'."
    else
      id "$user" &>/dev/null && sudo userdel "$user" && info "Removed user '${user}'."
      getent group "$group" &>/dev/null && sudo groupdel "$group" && info "Removed group '${group}'."
    fi
  fi
}

# =============================================================================
# utils_remove_dir DIR LABEL
# Interactively removes a directory after confirmation.
# =============================================================================
utils_remove_dir() {
  local dir="$1" label="$2"
  local remove="n"
  if [[ -d "$dir" ]]; then
    read -r -p "Remove ${label} '${dir}'? [y/N] " remove
    if [[ "$(echo "$remove" | tr '[:upper:]' '[:lower:]')" == "y" ]]; then
      sudo rm -rf "$dir"
      info "Removed ${dir}."
    fi
  fi
}

# =============================================================================
# mpi_params [SUMMARY]
# Computes optimal MPI parameters for the current machine and exports:
#   MPI_NP                — total number of processes (-np)
#   MPI_PPR               — processes per NUMA node (-map-by ppr:MPI_PPR:numa)
#   MPI_RAYON_NUM_THREADS — threads per process (-x RAYON_NUM_THREADS=…)
#
# SUMMARY: optional boolean (true/false). When true, prints the System
#          Configuration and MPI Parameters tables to stdout. Defaults to false.
#
# GPU grouping strategy (when GPUs are present):
#   - Prefer groups of 2 GPUs per process, else groups of 3, else 1 process per socket.
#   - If GPUs < sockets, fall back to 1 process total.
# =============================================================================
mpi_params() {
  local summary="${1:-false}"
  # Detect number of sockets (NUMA nodes)
  local num_sockets
  num_sockets=$(lscpu 2>/dev/null | grep "^Socket(s):" | awk '{print $2}')
  if [[ -z "$num_sockets" || "$num_sockets" -eq 0 ]]; then
    num_sockets=$(numactl --hardware 2>/dev/null | grep "available:" | awk '{print $2}')
  fi
  if [[ -z "$num_sockets" || "$num_sockets" -eq 0 ]]; then
    die "mpi_params: could not detect number of sockets/NUMA nodes."
  fi

  # Detect number of GPUs
  local num_gpus=0
  if command -v nvidia-smi &>/dev/null; then
    num_gpus=$(nvidia-smi -L 2>/dev/null | wc -l)
  fi

  # Detect total available threads
  local total_threads
  total_threads=$(nproc)

  # Calculate processes per socket
  local gpus_per_socket=0 procs_per_socket=1
  if [[ "$num_gpus" -gt 0 ]]; then
    if [[ $((num_gpus % num_sockets)) -ne 0 ]]; then
      info "mpi_params: warning: GPUs (${num_gpus}) don't divide evenly across sockets (${num_sockets})."
    fi
    gpus_per_socket=$((num_gpus / num_sockets))
    if [[ "$gpus_per_socket" -eq 0 ]]; then
      info "mpi_params: warning: fewer GPUs (${num_gpus}) than sockets (${num_sockets}), using 1 process total."
      procs_per_socket=0
    elif [[ $((gpus_per_socket % 2)) -eq 0 ]]; then
      procs_per_socket=$((gpus_per_socket / 2))
    elif [[ $((gpus_per_socket % 3)) -eq 0 ]]; then
      procs_per_socket=$((gpus_per_socket / 3))
    else
      procs_per_socket=1
    fi
  fi

  # Calculate NP and PPR
  local np ppr
  if [[ "$procs_per_socket" -eq 0 ]]; then
    np=1
    ppr=1
  else
    np=$((num_sockets * procs_per_socket))
    ppr=$procs_per_socket
  fi

  # Calculate GPUs per process (informational only)
  local gpus_per_process=0
  if [[ "$num_gpus" -gt 0 && "$np" -gt 0 ]]; then
    gpus_per_process=$(( num_gpus / np ))
  fi

  # Calculate RAYON_NUM_THREADS (at least 1)
  local rayon_num_threads=$(( total_threads / np ))
  [[ "$rayon_num_threads" -lt 1 ]] && rayon_num_threads=1

  if [[ "$summary" == "true" ]]; then
    echo "============================================"
    echo "System Configuration:"
    echo "============================================"
    echo "  Sockets (NUMA nodes): $num_sockets"
    echo "  GPUs:                 $num_gpus"
    echo "  GPUs per socket:      $gpus_per_socket"
    echo "  GPUs per process:     $gpus_per_process"
    echo "  Total threads:        $total_threads"
    echo ""
    echo "============================================"
    echo "MPI Parameters:"
    echo "============================================"
    echo "  Total processes (-np):           $np"
    echo "  Processes per NUMA (ppr):        $ppr"
    echo "  Threads per process (RAYON):     $rayon_num_threads"
    echo ""
  fi

  export MPI_NP=$np
  export MPI_PPR=$ppr
  export MPI_RAYON_NUM_THREADS=$rayon_num_threads
}
