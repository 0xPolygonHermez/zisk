#!/bin/bash

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
DEFAULT_WORKER_PROVINGKEY_DIR=""
DEFAULT_WORKER_EXTRA_ARGS=""


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
		ensure sudo dscl . -create "/Groups/${DEFAULT_SERVICE_GROUP}"
		ensure sudo dscl . -create "/Groups/${DEFAULT_SERVICE_GROUP}" PrimaryGroupID "$gid"
		ensure sudo dscl . -create "/Groups/${DEFAULT_SERVICE_GROUP}" RecordName "$DEFAULT_SERVICE_GROUP"
	else
		if getent group "$DEFAULT_SERVICE_GROUP" &>/dev/null; then
			info "Group '${DEFAULT_SERVICE_GROUP}' already exists, skipping."
			return 0
		fi

		ensure sudo groupadd --system "$DEFAULT_SERVICE_GROUP"
	fi
}

create_user_if_missing() {
	local service_realname="$1"

	info "Ensuring user '${DEFAULT_SERVICE_USER}' exists..."

	if [[ "$OS" == "Darwin" ]]; then
		if dscl . -read "/Users/${DEFAULT_SERVICE_USER}" &>/dev/null; then
			info "User '${DEFAULT_SERVICE_USER}' already exists, skipping."
			return 0
		fi

		local uid gid
		uid=$(( $(dscl . -list /Users UniqueID | awk '{print $2}' | sort -n | tail -1) + 1 ))
		gid=$(dscl . -read "/Groups/${DEFAULT_SERVICE_GROUP}" PrimaryGroupID | awk '{print $2}')

		ensure sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}"
		ensure sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" UniqueID "$uid"
		ensure sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" PrimaryGroupID "$gid"
		ensure sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" UserShell /usr/bin/false
		ensure sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" RealName "$service_realname"
		ensure sudo dscl . -create "/Users/${DEFAULT_SERVICE_USER}" NFSHomeDirectory /var/empty
	else
		if id "$DEFAULT_SERVICE_USER" &>/dev/null; then
			info "User '${DEFAULT_SERVICE_USER}' already exists, skipping."
			return 0
		fi

		ensure sudo useradd --system --gid "$DEFAULT_SERVICE_GROUP" --no-create-home --shell /usr/sbin/nologin "$DEFAULT_SERVICE_USER"
	fi
}

create_service_directories() {
	local data_dir="$1"
	local log_dir="$2"

	info "Creating service directories..."
	for dir in "$data_dir" "$log_dir"; do
		[[ -z "$dir" ]] && continue
		ensure sudo mkdir -p "$dir"
		ensure sudo chown "${DEFAULT_SERVICE_USER}:${DEFAULT_SERVICE_GROUP}" "$dir"
		ensure sudo chmod 0755 "$dir"
	done
}

install_binary_to_data_dir() {
	local bin_path="$1"
	local data_dir="$2"
	local bin_name="$3"

	[[ -x "$bin_path" ]] || {
		err "Binary not found or not executable: ${bin_path}"
		exit 1
	}

	local dest="${data_dir}/${bin_name}"
	info "Installing binary '${bin_path}' to '${dest}'..."
	if [[ "$OS" == "Darwin" ]]; then
		ensure sudo install -m 0755 -o root -g wheel "$bin_path" "$dest"
	else
		ensure sudo install -m 0755 -o root -g root "$bin_path" "$dest"
	fi
}

deploy_newsyslog_config() {
	local bin_name="$1"
	local log_dir="$2"

	local conf="/etc/newsyslog.d/${bin_name}.conf"

	info "Deploying newsyslog rotation config at ${conf}..."
	ensure sudo tee "$conf" > /dev/null <<NEWSYSLOG
# ${bin_name} log rotation
${log_dir}/${bin_name}.log  ${DEFAULT_SERVICE_USER}:${DEFAULT_SERVICE_GROUP}  640  ${DEFAULT_LOG_ROTATIONS}  $(( DEFAULT_LOG_MAX_SIZE_MB * 1024 ))  *  JG
NEWSYSLOG
	ensure sudo chmod 0644 "$conf"
}

deploy_systemd_unit() {
	local service_name="$1"
	local service_desc="$2"
	local data_dir="$3"
	local exec_start="$4"
	local service_nice="$5"

	local unit_file="/etc/systemd/system/${service_name}.service"
	local nice_line=""
	[[ -n "$service_nice" ]] && nice_line="Nice=${service_nice}"

	info "Deploying ${unit_file}..."
	ensure sudo tee "$unit_file" > /dev/null <<UNIT
[Unit]
Description=${service_desc}
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
WorkingDirectory=${data_dir}
${exec_start}
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
UNIT

	ensure sudo systemctl daemon-reload
	ensure sudo systemctl enable "$service_name"
	ensure sudo systemctl restart "$service_name"
}

deploy_launchd_plist() {
	local service_label="$1"
	local program_args="$2"
	local data_dir="$3"
	local bin_name="$4"
	local log_dir="$5"
	local service_nice="$6"

	local plist="/Library/LaunchDaemons/${service_label}.plist"

	local nice_block=""
	if [[ -n "$service_nice" ]]; then
		nice_block="    <key>Nice</key>
		<integer>${service_nice}</integer>
"
	fi

	info "Deploying ${plist}..."
	ensure sudo tee "$plist" > /dev/null <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
	"http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
		<key>Label</key>
		<string>${service_label}</string>

		<key>ProgramArguments</key>
${program_args}

		<key>UserName</key>
		<string>${DEFAULT_SERVICE_USER}</string>

		<key>GroupName</key>
		<string>${DEFAULT_SERVICE_GROUP}</string>

		<key>WorkingDirectory</key>
		<string>${data_dir}</string>

		<key>KeepAlive</key>
		<false/>

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
PLIST

	ensure sudo chown root:wheel "$plist"
	ensure sudo chmod 0644 "$plist"
	sudo launchctl unload "$plist" 2>/dev/null || true
	ensure sudo launchctl load -w "$plist"
}

remove_service_dir_if_present() {
	local dir="$1"
	local label="$2"

	[[ -z "$dir" ]] && return 0
	if [[ -d "$dir" ]]; then
		ensure sudo rm -rf "$dir"
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

	# kill all journalctl processes
	info "Killing existing journalctl processes..."
	pkill -f journalctl || true

	if [[ "$OS" == "Darwin" ]]; then
		if [[ ! -f "$plist" ]]; then
			warn "${service_name} is not installed (${plist} not found). Skipping uninstall."
		else
			info "Uninstalling ${service_name}..."
			sudo launchctl unload "$plist" 2>/dev/null || true
			ensure sudo rm -f "$plist"
			info "Removed ${plist}."

			local newsyslog_conf="/etc/newsyslog.d/${service_name}.conf"
			if [[ -f "$newsyslog_conf" ]]; then
				ensure sudo rm -f "$newsyslog_conf"
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
			ensure sudo rm -f "$unit"
			ensure sudo systemctl daemon-reload
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
	local service_name="$DEFAULT_COORDINATOR_BIN_NAME"
	local service_label="com.zisk.coordinator"
	local service_desc="Zisk Coordinator"
	local service_realname="Zisk Coordinator"
	local bin_path="$DEFAULT_COORDINATOR_BIN_PATH"
	local bin_name="$DEFAULT_COORDINATOR_BIN_NAME"
	local data_dir="$DEFAULT_COORDINATOR_DATA_DIR"
	local log_dir="$DEFAULT_COORDINATOR_LOG_DIR"
	local exec_start
	exec_start="$(build_coordinator_exec_start)"
	local program_args
	program_args="$(build_coordinator_program_args_plist)"
	local service_nice="-10"

	create_group_if_missing
	create_user_if_missing "$service_realname"
	create_service_directories "$data_dir" "$log_dir"
	install_binary_to_data_dir "$bin_path" "$data_dir" "$bin_name"

	if [[ "$OS" == "Darwin" ]]; then
		deploy_launchd_plist "$service_label" "$program_args" "$data_dir" "$bin_name" "$log_dir" "$service_nice"
		deploy_newsyslog_config "$bin_name" "$log_dir"
	else
		deploy_systemd_unit "$service_name" "$service_desc" "$data_dir" "$exec_start" ""
	fi
}

# =============================================================================
# Worker-specific helpers
# =============================================================================
build_worker_exec_start() {
	local hints_arg=""
	local gpu_arg=""

	local common_args="--coordinator-url ${DEFAULT_WORKER_COORDINATOR_URL} -m"
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

	args+=(--coordinator-url "${DEFAULT_WORKER_COORDINATOR_URL}" --worker-id "${DEFAULT_WORKER_ID}" -m)
	[[ -n "$DEFAULT_WORKER_PROVINGKEY_DIR" ]] && args+=(-k "$DEFAULT_WORKER_PROVINGKEY_DIR")

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
	local service_name="$DEFAULT_WORKER_BIN_NAME"
	local service_label="com.zisk.worker"
	local service_desc="Zisk Worker"
	local service_realname="Zisk Worker"
	local bin_path="$DEFAULT_WORKER_BIN_PATH"
	local bin_name="$DEFAULT_WORKER_BIN_NAME"
	local data_dir="$DEFAULT_WORKER_DATA_DIR"
	local log_dir="$DEFAULT_WORKER_LOG_DIR"
	local exec_start
	exec_start="$(build_worker_exec_start)"
	local program_args
	program_args="$(build_worker_program_args_plist)"
	local service_nice="-10"

	create_group_if_missing
	create_user_if_missing "$service_realname"
	create_service_directories "$data_dir" "$log_dir"
	install_binary_to_data_dir "$bin_path" "$data_dir" "$bin_name"

	if [[ "$OS" == "Darwin" ]]; then
		deploy_launchd_plist "$service_label" "$program_args" "$data_dir" "$bin_name" "$log_dir" "$service_nice"
		deploy_newsyslog_config "$bin_name" "$log_dir"
	else
		deploy_systemd_unit "$service_name" "$service_desc" "$data_dir" "$exec_start" ""
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

uninstall_distributed() {
	cleanup_existing_services
	sleep 3 # Wait a moment to ensure all processes have been stopped
	info "zisk-coordinator and zisk-worker services have been uninstalled."
}
