#!/usr/bin/env bash
set -euo pipefail

source "./utils.sh"

OS="$(uname -s)"

# =============================================================================
# Defaults (hardcoded, easy to change later)
# =============================================================================
DEFAULT_SERVICE_USER="$(id -un)"
DEFAULT_SERVICE_GROUP="$(id -gn)"

DEFAULT_COORDINATOR_BIN_NAME="zisk-coordinator"
DEFAULT_WORKER_BIN_NAME="zisk-worker"

DEFAULT_COORDINATOR_BIN_PATH="${HOME}/.zisk/bin/${DEFAULT_COORDINATOR_BIN_NAME}"
DEFAULT_WORKER_BIN_PATH="${HOME}/.zisk/bin/${DEFAULT_WORKER_BIN_NAME}"

DEFAULT_COORDINATOR_DATA_DIR="/var/lib/${DEFAULT_COORDINATOR_BIN_NAME}"
DEFAULT_COORDINATOR_LOG_DIR="/var/log/${DEFAULT_COORDINATOR_BIN_NAME}"
DEFAULT_COORDINATOR_API_PORT="7010"
DEFAULT_COORDINATOR_CLUSTER_PORT="6100"

DEFAULT_WORKER_DATA_DIR="/var/lib/${DEFAULT_WORKER_BIN_NAME}"
DEFAULT_WORKER_LOG_DIR="/var/log/${DEFAULT_WORKER_BIN_NAME}"
DEFAULT_WORKER_COORDINATOR_URL="http://127.0.0.1:${DEFAULT_COORDINATOR_CLUSTER_PORT}"

DEFAULT_LOG_MAX_SIZE_MB="100"
DEFAULT_LOG_ROTATIONS="5"

# Worker hardcoded runtime params (same behavior as deploy_worker default call)
DEFAULT_WORKER_ID="worker-01"
DEFAULT_WORKER_NO_MPI="true"
DEFAULT_WORKER_HINTS_ENABLED="false"
DEFAULT_WORKER_GPU_ENABLED="false"
DEFAULT_WORKER_PROVINGKEY_DIR=""
DEFAULT_WORKER_EXTRA_ARGS=""

# Active service context (set before invoking shared helpers)
SERVICE_NAME=""
SERVICE_LABEL=""
SERVICE_DESC=""
SERVICE_REALNAME=""
SERVICE_BIN_PATH=""
SERVICE_BIN_NAME=""
SERVICE_DATA_DIR=""
SERVICE_LOG_DIR=""
SERVICE_EXEC_START=""
SERVICE_PROGRAM_ARGS=""
SERVICE_NICE=""

# =============================================================================
# Generic helpers
# =============================================================================
ensure_supported_os() {
	case "$OS" in
		Darwin|Linux)
			;;
		*)
			err "Unsupported OS: ${OS}. Only macOS and Ubuntu/Linux are supported."
			exit 1
			;;
	esac
}

create_group_if_missing() {
	info "Ensuring group '${DEFAULT_SERVICE_GROUP}' exists..."

	if [[ "$OS" == "Darwin" ]]; then
		if dscl . -read "/Groups/${DEFAULT_SERVICE_GROUP}" &>/dev/null; then
			info "Group '${DEFAULT_SERVICE_GROUP}' already exists, skipping."
			return 0
		fi

		local gid
		gid=$(( $(dscl . -list /Groups PrimaryGroupID | awk '{print $2}' | sort -n | tail -1) + 1 ))
		sudo dscl . -create "/Groups/${DEFAULT_SERVICE_GROUP}"
		sudo dscl . -create "/Groups/${DEFAULT_SERVICE_GROUP}" PrimaryGroupID "$gid"
		sudo dscl . -create "/Groups/${DEFAULT_SERVICE_GROUP}" RecordName "$DEFAULT_SERVICE_GROUP"
	else
		if getent group "$DEFAULT_SERVICE_GROUP" &>/dev/null; then
			info "Group '${DEFAULT_SERVICE_GROUP}' already exists, skipping."
			return 0
		fi

		sudo groupadd --system "$DEFAULT_SERVICE_GROUP"
	fi
}

create_user_if_missing() {
	info "Ensuring user '${DEFAULT_SERVICE_USER}' exists..."

	if [[ "$OS" == "Darwin" ]]; then
		if dscl . -read "/Users/${DEFAULT_SERVICE_USER}" &>/dev/null; then
			info "User '${DEFAULT_SERVICE_USER}' already exists, skipping."
			return 0
		fi

		local uid gid
		uid=$(( $(dscl . -list /Users UniqueID | awk '{print $2}' | sort -n | tail -1) + 1 ))
		gid=$(dscl . -read "/Groups/${DEFAULT_SERVICE_GROUP}" PrimaryGroupID | awk '{print $2}')

		sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}"
		sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" UniqueID "$uid"
		sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" PrimaryGroupID "$gid"
		sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" UserShell /usr/bin/false
		sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" RealName "$SERVICE_REALNAME"
		sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" NFSHomeDirectory /var/empty
	else
		if id "$DEFAULT_SERVICE_USER" &>/dev/null; then
			info "User '${DEFAULT_SERVICE_USER}' already exists, skipping."
			return 0
		fi

		sudo useradd --system --gid "$DEFAULT_SERVICE_GROUP" --no-create-home --shell /usr/sbin/nologin "$DEFAULT_SERVICE_USER"
	fi
}

create_service_directories() {
	info "Creating service directories..."
	for dir in "$SERVICE_DATA_DIR" "$SERVICE_LOG_DIR"; do
		[[ -z "$dir" ]] && continue
		sudo mkdir -p "$dir"
		sudo chown "${DEFAULT_SERVICE_USER}:${DEFAULT_SERVICE_GROUP}" "$dir"
		sudo chmod 0755 "$dir"
	done
}

install_binary_to_data_dir() {
	[[ -x "$SERVICE_BIN_PATH" ]] || {
		err "Binary not found or not executable: ${SERVICE_BIN_PATH}"
		exit 1
	}

	local dest="${SERVICE_DATA_DIR}/${SERVICE_BIN_NAME}"
	info "Installing binary '${SERVICE_BIN_PATH}' to '${dest}'..."
	if [[ "$OS" == "Darwin" ]]; then
		sudo install -m 0755 -o root -g wheel "$SERVICE_BIN_PATH" "$dest"
	else
		sudo install -m 0755 -o root -g root "$SERVICE_BIN_PATH" "$dest"
	fi
}

deploy_newsyslog_config() {
	local conf="/etc/newsyslog.d/${SERVICE_BIN_NAME}.conf"

	info "Deploying newsyslog rotation config at ${conf}..."
	sudo tee "$conf" > /dev/null <<NEWSYSLOG
# ${SERVICE_BIN_NAME} log rotation
${SERVICE_LOG_DIR}/${SERVICE_BIN_NAME}.log  ${DEFAULT_SERVICE_USER}:${DEFAULT_SERVICE_GROUP}  640  ${DEFAULT_LOG_ROTATIONS}  $(( DEFAULT_LOG_MAX_SIZE_MB * 1024 ))  *  JG
NEWSYSLOG
	sudo chmod 0644 "$conf"
}

deploy_systemd_unit() {
	local unit_file="/etc/systemd/system/${SERVICE_NAME}.service"
	local nice_line=""
	[[ -n "$SERVICE_NICE" ]] && nice_line="Nice=${SERVICE_NICE}"

	info "Deploying ${unit_file}..."
	sudo tee "$unit_file" > /dev/null <<UNIT
[Unit]
Description=${SERVICE_DESC}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
Environment=ZISK_USE_INSTALLED=1
User=${DEFAULT_SERVICE_USER}
Group=${DEFAULT_SERVICE_GROUP}
Restart=no
LimitNOFILE=65535
${nice_line}
WorkingDirectory=${SERVICE_DATA_DIR}
${SERVICE_EXEC_START}
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
UNIT

	sudo systemctl daemon-reload
	sudo systemctl enable "$SERVICE_NAME"
	sudo systemctl restart "$SERVICE_NAME"
}

deploy_launchd_plist() {
	local plist="/Library/LaunchDaemons/${SERVICE_LABEL}.plist"

	local nice_block=""
	if [[ -n "$SERVICE_NICE" ]]; then
		nice_block="    <key>Nice</key>
		<integer>${SERVICE_NICE}</integer>
"
	fi

	info "Deploying ${plist}..."
	sudo tee "$plist" > /dev/null <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
	"http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
		<key>Label</key>
		<string>${SERVICE_LABEL}</string>

		<key>ProgramArguments</key>
${SERVICE_PROGRAM_ARGS}

		<key>UserName</key>
		<string>${DEFAULT_SERVICE_USER}</string>

		<key>GroupName</key>
		<string>${DEFAULT_SERVICE_GROUP}</string>

		<key>WorkingDirectory</key>
		<string>${SERVICE_DATA_DIR}</string>

		<key>KeepAlive</key>
		<false/>

		<key>StandardOutPath</key>
		<string>${SERVICE_LOG_DIR}/${SERVICE_BIN_NAME}.log</string>

		<key>StandardErrorPath</key>
		<string>${SERVICE_LOG_DIR}/${SERVICE_BIN_NAME}.log</string>

		<key>ProcessType</key>
		<string>Interactive</string>

${nice_block}    <key>SoftResourceLimits</key>
		<dict>
				<key>NumberOfFiles</key>
				<integer>65535</integer>
		</dict>
</dict>
</plist>
PLIST

	sudo chown root:wheel "$plist"
	sudo chmod 0644 "$plist"
	sudo launchctl unload "$plist" 2>/dev/null || true
	sudo launchctl load -w "$plist"
}

remove_service_dir_if_present() {
	local dir="$1"
	local label="$2"

	[[ -z "$dir" ]] && return 0
	if [[ -d "$dir" ]]; then
		sudo rm -rf "$dir"
		info "Removed ${label} '${dir}'."
	fi
}

cleanup_existing_service() {
	local service_name="$1"
	local service_label="$2"
	local data_dir="$3"
	local log_dir="$4"

	local plist="/Library/LaunchDaemons/${service_label}.plist"
	local unit="/etc/systemd/system/${service_name}.service"

	if [[ "$OS" == "Darwin" ]]; then
		if [[ ! -f "$plist" ]]; then
			warn "${service_name} is not installed (${plist} not found). Skipping uninstall."
		else
			info "Uninstalling ${service_name}..."
			sudo launchctl unload "$plist" 2>/dev/null || true
			sudo rm -f "$plist"
			info "Removed ${plist}."

			local newsyslog_conf="/etc/newsyslog.d/${service_name}.conf"
			if [[ -f "$newsyslog_conf" ]]; then
				sudo rm -f "$newsyslog_conf"
				info "Removed ${newsyslog_conf}."
			fi
		fi
	else
		if [[ ! -f "$unit" ]]; then
			warn "${service_name} is not installed (${unit} not found). Skipping uninstall."
		else
			info "Uninstalling ${service_name}..."
			if systemctl list-unit-files "${service_name}.service" &>/dev/null; then
				info "Stopping and disabling systemd service..."
				sudo systemctl stop "$service_name" 2>/dev/null || true
				sudo systemctl disable "$service_name" 2>/dev/null || true
			fi
			sudo rm -f "$unit"
			sudo systemctl daemon-reload
			info "Removed ${unit}."
		fi
	fi

	remove_service_dir_if_present "$log_dir" "log directory"
	remove_service_dir_if_present "$data_dir" "data directory"
}

cleanup_existing_services() {
	cleanup_existing_service "$DEFAULT_COORDINATOR_BIN_NAME" "com.zisk.coordinator" "$DEFAULT_COORDINATOR_DATA_DIR" "$DEFAULT_COORDINATOR_LOG_DIR"
	cleanup_existing_service "$DEFAULT_WORKER_BIN_NAME" "com.zisk.worker" "$DEFAULT_WORKER_DATA_DIR" "$DEFAULT_WORKER_LOG_DIR"
}

# =============================================================================
# Coordinator-specific helpers
# =============================================================================
build_coordinator_exec_start() {
	printf "ExecStart=%s --api-port %s --cluster-port %s\n" "${DEFAULT_COORDINATOR_DATA_DIR}/${DEFAULT_COORDINATOR_BIN_NAME}" "$DEFAULT_COORDINATOR_API_PORT" "$DEFAULT_COORDINATOR_CLUSTER_PORT"
}

build_coordinator_program_args_plist() {
	local args=("${DEFAULT_COORDINATOR_DATA_DIR}/${DEFAULT_COORDINATOR_BIN_NAME}" --api-port "$DEFAULT_COORDINATOR_API_PORT" --cluster-port "$DEFAULT_COORDINATOR_CLUSTER_PORT")

	printf "    <array>\n"
	for arg in "${args[@]}"; do
		printf "        <string>%s</string>\n" "$arg"
	done
	printf "    </array>\n"
}

deploy_coordinator_service() {
	SERVICE_NAME="$DEFAULT_COORDINATOR_BIN_NAME"
	SERVICE_LABEL="com.zisk.coordinator"
	SERVICE_DESC="Zisk Coordinator"
	SERVICE_REALNAME="Zisk Coordinator"
	SERVICE_BIN_PATH="$DEFAULT_COORDINATOR_BIN_PATH"
	SERVICE_BIN_NAME="$DEFAULT_COORDINATOR_BIN_NAME"
	SERVICE_DATA_DIR="$DEFAULT_COORDINATOR_DATA_DIR"
	SERVICE_LOG_DIR="$DEFAULT_COORDINATOR_LOG_DIR"
	SERVICE_EXEC_START="$(build_coordinator_exec_start)"
	SERVICE_PROGRAM_ARGS="$(build_coordinator_program_args_plist)"
	SERVICE_NICE="-10"

	create_group_if_missing
	create_user_if_missing
	create_service_directories
	install_binary_to_data_dir

	if [[ "$OS" == "Darwin" ]]; then
		deploy_launchd_plist
		deploy_newsyslog_config
	else
		SERVICE_NICE=""
		deploy_systemd_unit
	fi
}

# =============================================================================
# Worker-specific helpers
# =============================================================================
build_worker_exec_start() {
	local hints_arg=""
	local gpu_arg=""
	[[ "$DEFAULT_WORKER_HINTS_ENABLED" == "true" ]] && hints_arg=" --hints"
	[[ "$DEFAULT_WORKER_GPU_ENABLED" == "true" ]] && gpu_arg=" --gpu"

	local common_args="--coordinator-url ${DEFAULT_WORKER_COORDINATOR_URL}"
	[[ -n "$DEFAULT_WORKER_PROVINGKEY_DIR" ]] && common_args+=" -k ${DEFAULT_WORKER_PROVINGKEY_DIR}"
	common_args+=" --worker-id ${DEFAULT_WORKER_ID}${hints_arg}${gpu_arg}"
	[[ "${ONLY_CPU:-}" != "1" ]] && common_args+=" --gpu"
	[[ -n "$DEFAULT_WORKER_EXTRA_ARGS" ]] && common_args+=" ${DEFAULT_WORKER_EXTRA_ARGS}"


	if [[ "$DEFAULT_WORKER_NO_MPI" == "true" ]]; then
		printf "ExecStart=%s %s\n" "${DEFAULT_WORKER_DATA_DIR}/${DEFAULT_WORKER_BIN_NAME}" "$common_args"
	else
		# Hardcoded MPI params for ethproofs test flow.
		local mpi_processes="1"
		local mpi_ppr_numa="1"
		local mpi_threads="1"
		printf "ExecStart=mpirun --report-bindings --allow-run-as-root -np %s -map-by ppr:%s:numa --bind-to numa --rank-by slot -x RAYON_NUM_THREADS=%s %s %s\n" "$mpi_processes" "$mpi_ppr_numa" "$mpi_threads" "${DEFAULT_WORKER_DATA_DIR}/${DEFAULT_WORKER_BIN_NAME}" "$common_args"
	fi
}

build_worker_program_args_plist() {
	local args=()
	if [[ "$DEFAULT_WORKER_NO_MPI" == "true" ]]; then
		args+=("${DEFAULT_WORKER_DATA_DIR}/${DEFAULT_WORKER_BIN_NAME}")
	else
		args+=(mpirun --report-bindings --allow-run-as-root -np "1" -map-by "ppr:1:numa" --bind-to numa --rank-by slot -x "RAYON_NUM_THREADS=1" "${DEFAULT_WORKER_DATA_DIR}/${DEFAULT_WORKER_BIN_NAME}")
	fi

	args+=(--coordinator-url "${DEFAULT_WORKER_COORDINATOR_URL}" --worker-id "${DEFAULT_WORKER_ID}")
	[[ -n "$DEFAULT_WORKER_PROVINGKEY_DIR" ]] && args+=(-k "$DEFAULT_WORKER_PROVINGKEY_DIR")
	[[ "$DEFAULT_WORKER_HINTS_ENABLED" == "true" ]] && args+=(--hints)
	[[ "$DEFAULT_WORKER_GPU_ENABLED" == "true" ]] && args+=(--gpu)

	if [[ -n "$DEFAULT_WORKER_EXTRA_ARGS" ]]; then
		read -ra extra_arr <<< "$DEFAULT_WORKER_EXTRA_ARGS"
		args+=("${extra_arr[@]}")
	fi

	printf "    <array>\n"
	for arg in "${args[@]}"; do
		printf "        <string>%s</string>\n" "$arg"
	done
	printf "    </array>\n"
}

deploy_worker_service() {
	SERVICE_NAME="$DEFAULT_WORKER_BIN_NAME"
	SERVICE_LABEL="com.zisk.worker"
	SERVICE_DESC="Zisk Worker"
	SERVICE_REALNAME="Zisk Worker"
	SERVICE_BIN_PATH="$DEFAULT_WORKER_BIN_PATH"
	SERVICE_BIN_NAME="$DEFAULT_WORKER_BIN_NAME"
	SERVICE_DATA_DIR="$DEFAULT_WORKER_DATA_DIR"
	SERVICE_LOG_DIR="$DEFAULT_WORKER_LOG_DIR"
	SERVICE_EXEC_START="$(build_worker_exec_start)"
	SERVICE_PROGRAM_ARGS="$(build_worker_program_args_plist)"
	SERVICE_NICE="-10"

	create_group_if_missing
	create_user_if_missing
	create_service_directories
	install_binary_to_data_dir

	if [[ "$OS" == "Darwin" ]]; then
		deploy_launchd_plist
		deploy_newsyslog_config
	else
		deploy_systemd_unit
	fi
}

# prefix_log_output: Prefix each log line written to stdout.
prefix_log_output() {
    local prefix="$1"
    local prefix_width="${LOG_PREFIX_WIDTH:-11}"

    awk -v prefix="${prefix}" -v width="${prefix_width}" '{ printf "[%-*s] %s\n", width, prefix, $0; fflush() }'
}

deploy_distributed() {
    local startup_wait="20"
    local startup_since
    startup_since="$(date '+%Y-%m-%d %H:%M:%S')"

	ensure_supported_os
	cleanup_existing_services

	info "Deploying zisk-coordinator service..."
	deploy_coordinator_service

	info "Deploying zisk-worker service..."
	deploy_worker_service

	# kill all journalctl processes
	info "Killing existing journalctl processes..."
	pkill -f journalctl || true

    # Stream service logs to stdout in background
    journalctl -fu zisk-coordinator 2>/dev/null | prefix_log_output "coordinator" &
    local log_coord_pid=$!
    journalctl -fu zisk-worker 2>/dev/null | prefix_log_output "worker" &
    local log_worker_pid=$!

    info "Waiting for worker to register (timeout: ${startup_wait}s)..."
    local startup_elapsed=0
    while [[ ${startup_elapsed} -lt ${startup_wait} ]]; do
        if journalctl -u zisk-worker --since "${startup_since}" --no-pager 2>/dev/null | grep -qF "Registration accepted: Registration successful"; then
            info "Worker registered successfully."
            break
        fi
        if ! systemctl is-active --quiet zisk-coordinator; then
            kill "${log_coord_pid}" "${log_worker_pid}" 2>/dev/null || true
            sudo systemctl stop zisk-coordinator zisk-worker 2>/dev/null || true
            err "zisk-coordinator service stopped during startup."
            return 1
        fi
        if ! systemctl is-active --quiet zisk-worker; then
            kill "${log_coord_pid}" "${log_worker_pid}" 2>/dev/null || true
            sudo systemctl stop zisk-coordinator zisk-worker 2>/dev/null || true
            err "zisk-worker service stopped during startup."
            return 1
        fi
        sleep 2
        startup_elapsed=$(( startup_elapsed + 2 ))
    done
    if [[ ${startup_elapsed} -ge ${startup_wait} ]]; then
        kill "${log_coord_pid}" "${log_worker_pid}" 2>/dev/null || true
        sudo systemctl stop zisk-coordinator zisk-worker 2>/dev/null || true
        err "Worker did not register within ${startup_wait}s."
        return 1
    fi

	success "zisk-coordinator and zisk-worker services have been deployed."
}
