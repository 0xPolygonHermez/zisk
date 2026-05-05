#!/bin/bash

source "./utils.sh"

PROOF_DIR="./proof"

# print_proofs_result: Display proof results in a table format
#
# Parameters:
#   $1 (base_path) — Directory path where result JSON files are stored
#   $2…$n (files)  — Input filenames (without “.json”) of the result files to include in the table
#
# Example:
#   print_proofs_result "/home/user/work/proofs/distributed" file1 file2 file3
print_proofs_result() {
    local base_path="$1"
    shift
    local files=("$@")

    # Header
    printf "| %-35s | %-10s | %-15s |\n" "-----------------------------------" "----------" "---------------"
    printf "| %-35s | %-10s | %-15s |\n" "File"                           "Time (s)"   "Cycles"
    printf "| %-35s | %-10s | %-15s |\n" "-----------------------------------" "----------" "---------------"

    for f in "${files[@]}"; do
        local fullpath="${base_path}/${f}.json"

        if [[ ! -f "$fullpath" ]]; then
            printf "| %-35s | %-10s | %-15s |\n" "$f" "N/A" "N/A"
            continue
        fi

        # Extract raw time and drop fractional part (portable sed on macOS/Linux)
        # Matches: "time": 12.345
        local raw_time
        raw_time=$(sed -nE 's/.*"time"[[:space:]]*:[[:space:]]*([0-9.]+).*/\1/p' "$fullpath")
        local time_int="${raw_time%%.*}"

        # Extract cycles: integer after "cycles":
        local cycles
        cycles=$(sed -nE 's/.*"cycles"[[:space:]]*:[[:space:]]*([0-9]+).*/\1/p' "$fullpath")


        printf "| %-35s | %-10s | %-15s |\n" "$f" "$time_int" "$cycles"
    done

    printf "| %-35s | %-10s | %-15s |\n" "-----------------------------------" "----------" "---------------"

    echo
}

# resolve_verify_proof_file: Pick proof_*.bin in ./proof for verification.
# Falls back to ./proof/vadcop_final_proof.bin for backward compatibility.
resolve_verify_proof_file() {
    local selected_file
    selected_file=$(find "${PROOF_DIR}" -maxdepth 1 -type f -name "proof_*.bin" | sort | head -n 1)
    if [[ -n "${selected_file}" ]]; then
        echo "${selected_file}"
        return 0
    fi

    if [[ -f "${PROOF_DIR}/vadcop_final_proof.bin" ]]; then
        echo "${PROOF_DIR}/vadcop_final_proof.bin"
        return 0
    fi

    return 1
}

# prefix_log_output: Prefix each log line written to stdout.
prefix_log_output() {
    local prefix="$1"
    local prefix_width="${LOG_PREFIX_WIDTH:-11}"

    awk -v prefix="${prefix}" -v width="${prefix_width}" '{ printf "[%-*s] %s\n", width, prefix, $0; fflush() }'
}

# kill_distributed: Reset INT/TERM trap and kill both coordinator and worker.
kill_distributed() {
    info "Stopping zisk-coordinator and zisk-worker processes..."
    trap - INT TERM
    pkill -9 -u "$(id -u)" -f "(^|/)zisk-worker([[:space:]]|$)" 2>/dev/null || true
    pkill -9 -u "$(id -u)" -x "zisk-worker" 2>/dev/null || true
    sleep 3
    pkill -9 -u "$(id -u)" -f "(^|/)zisk-coordinator([[:space:]]|$)" 2>/dev/null || true
    pkill -9 -u "$(id -u)" -x "zisk-coordinato" 2>/dev/null || true
    info "Processes stopped."
}

# distributed_prove: Prove an ELF program in distributed mode (zisk-coordinator + zisk-worker).
#
# Starts a coordinator and a single worker in the background, submits the proof
# request via "zisk-coordinator prove", polls for completion, copies the proof
# files to the expected output directory, then stops both processes.
#
# Parameters:
#   $1 (elf_file)    – Path to the ELF binary (absolute or relative to cwd)
#   $2 (inputs_path) – Directory containing input files
#   $3 (input_file)  – Input filename (basename only) or "empty" for no input
distributed_prove() {
    local elf_file="$1"
    local inputs_path="$2"
    local input_file="$3"
    local log_prefix="$4"

    local port="${COORDINATOR_PORT:-50051}"
    local capacity="${COORDINATOR_COMPUTE_CAPACITY:-10}"
    local startup_wait="${COORDINATOR_STARTUP_WAIT:-120}"
    local prove_timeout="${COORDINATOR_PROVE_TIMEOUT:-3600}"
    local coord_url="http://127.0.0.1:${port}"

    local coord_pid worker_pid

    LOGS_DIR="${WORKSPACE_DIR}/logs"

    rm -rf "${PROOF_DIR}"

    # Kill any leftover zisk-coordinator / zisk-worker processes owned by the current user
    kill_distributed

    info "Starting zisk-coordinator on port ${port}..."
    zisk-coordinator \
        --port "${port}" \
        --proofs-dir "${PROOF_DIR}" \
        --compressed-proofs \
        2>&1 | tee "${LOGS_DIR}/distributed/coordinator.log" | prefix_log_output "coordinator" &
    coord_pid=$!

    info "Starting zisk-worker (elf: ${elf_file}, inputs-folder: ${inputs_path})..."
    zisk-worker \
        --elf "${elf_file}" \
        --inputs-folder "${inputs_path}" \
        --compute-capacity "${capacity}" \
        --coordinator-url "${coord_url}" \
        --asm-port 6100 \
        2>&1 | tee "${LOGS_DIR}/distributed/worker.log" | prefix_log_output "worker" &
    worker_pid=$!

    # Terminate background processes if the script is interrupted (Ctrl-C / SIGTERM)
    trap 'trap - INT TERM; kill "${coord_pid}" "${worker_pid}" 2>/dev/null || true; wait "${coord_pid}" "${worker_pid}" 2>/dev/null || true; exit 130' INT TERM

    info "Waiting for worker to register (timeout: ${startup_wait}s)..."
    local startup_elapsed=0
    while [[ ${startup_elapsed} -lt ${startup_wait} ]]; do
        if grep -qF "Registration accepted: Registration successful" "${LOGS_DIR}/distributed/worker.log" 2>/dev/null; then
            info "Worker registered successfully."
            break
        fi
        if ! kill -0 "${coord_pid}" 2>/dev/null; then
            kill_distributed
            err "Coordinator exited during startup. See ${LOGS_DIR}/distributed/coordinator.log"
            return 1
        fi
        if ! kill -0 "${worker_pid}" 2>/dev/null; then
            kill_distributed
            err "Worker exited during startup. See ${LOGS_DIR}/distributed/worker.log"
            return 1
        fi
        sleep 2
        startup_elapsed=$(( startup_elapsed + 2 ))
    done
    if [[ ${startup_elapsed} -ge ${startup_wait} ]]; then
        kill_distributed
        err "Worker did not register within ${startup_wait}s. See ${LOGS_DIR}/distributed/worker.log"
        return 1
    fi

    # Build --inputs-uri flag with full path (omit entirely when input is "empty").
    local inputs_flag=""
    if [[ "${input_file}" != "empty" ]]; then
        inputs_flag="--inputs-uri ${inputs_path}/${input_file} --direct-inputs"
    fi

    info "Submitting proof via zisk-coordinator prove..."
    zisk-coordinator prove \
        --coordinator-url "${coord_url}" \
        ${inputs_flag} \
        --compute-capacity "${capacity}" \
        >"${LOGS_DIR}/single/prove_${input_file}.log" 2>&1
    local prove_exit=$?

    if [[ ${prove_exit} -ne 0 ]]; then
        kill_distributed
        err "zisk-coordinator prove failed (exit ${prove_exit}). See ${LOGS_DIR}/single/prove_${input_file}.log"
        return 1
    fi

    # Poll for proof completion via worker logs
    local elapsed=0
    local prove_completed=false
    info "Waiting for proof to complete (timeout: ${prove_timeout}s)..."
    while [[ ${elapsed} -lt ${prove_timeout} ]]; do
        if grep -qF "Aggregation task completed" "${LOGS_DIR}/distributed/worker.log" 2>/dev/null; then
            prove_completed=true
            break
        fi
        if ! kill -0 "${worker_pid}" 2>/dev/null; then
            kill_distributed
            err "Worker process (pid=${worker_pid}) exited unexpectedly. See ${LOGS_DIR}/distributed/worker.log"
            return 1
        fi
        sleep 5
        elapsed=$(( elapsed + 5 ))
    done

    kill_distributed

    if [[ "${prove_completed}" != "true" ]]; then
        err "Proof did not complete within ${prove_timeout}s."
        err "  Coordinator log : ${LOGS_DIR}/distributed/coordinator.log"
        err "  Worker log      : ${LOGS_DIR}/distributed/worker.log"
        return 1
    fi

    # Find the proof file produced by the coordinator
    local proof_file
    proof_file=$(resolve_verify_proof_file) || {
        err "No proof_*.bin or vadcop_final_proof.bin found in ${PROOF_DIR}"
        return 1
    }

    # # Copy proof files to the expected location so downstream steps are identical
    # cp "${proof_file}" "${PROOF_DIR}/vadcop_final_proof.bin"
    # local proof_job_dir
    # proof_job_dir=$(dirname "${proof_file}")
    # if [[ -f "${proof_job_dir}/result.json" ]]; then
    #     cp "${proof_job_dir}/result.json" "${PROOF_DIR}/result.json"
    # fi

    info "Proof completed: ${proof_file}"
    return 0
}

# test_elf: Run proofs for a given ELF program.
#
# Parameters:
#   $1 (elf_file)      – Path to the ELF binary
#   $2 (inputs_path)   – Directory where input files are located
#   $3 (inputs_prefix) – Prefix for input env variables.
#                        The function appends _SINGLE, _MPI, _DISTRIBUTED to derive
#                        the actual variable names (e.g. BLOCK_INPUTS_SINGLE)
#   $4 (desc)          – Descriptive label for logging
#
# Each proving mode is enabled by populating the corresponding input variable:
#   <PREFIX>_SINGLE      — non-empty → runs "cargo-zisk prove" (no mpirun)
#   <PREFIX>_MPI         — non-empty → runs "cargo-zisk prove" via mpirun
#   <PREFIX>_DISTRIBUTED — non-empty → starts zisk-coordinator + zisk-worker
# Leave any variable empty to skip that mode.
#
# Example:
#  test_elf "program.elf" "inputs" "BLOCK_INPUTS" "Ethereum blocks"
test_elf() {
    local elf_file="$1"
    local inputs_path="$2"
    local inputs_prefix="$3"
    local desc="$4"

    current_dir=$(pwd)

    info "Executing ${desc} script"

    is_proving_key_installed || return 1

    export ELF_FILE="$elf_file"
    export INPUTS_PATH="$inputs_path"

    # Derive input variable names from prefix
    local single_var="${inputs_prefix}_SINGLE"
    local mpi_var="${inputs_prefix}_MPI"
    local dist_var="${inputs_prefix}_DISTRIBUTED"

    export INPUTS_SINGLE="${!single_var}"
    export INPUTS_MPI="${!mpi_var}"
    export INPUTS_DISTRIBUTED="${!dist_var}"

    # Validate required binaries if distributed inputs are provided
    if [[ -n "${INPUTS_DISTRIBUTED}" ]]; then
        if ! command -v zisk-coordinator &>/dev/null; then
            err "zisk-coordinator binary not found in PATH. Required for distributed proving"
            return 1
        fi
        if ! command -v zisk-worker &>/dev/null; then
            err "zisk-worker binary not found in PATH. Required for distributed proving"
            return 1
        fi
    fi

    declare -a result_files=() result_mpi_files=() result_dist_files=()
    declare -a inputs=() mpi_inputs=() dist_inputs=()

    # Load all input arrays; a non-empty list enables that proving mode
    get_var_list_to_array inputs      "INPUTS_SINGLE"
    get_var_list_to_array mpi_inputs  "INPUTS_MPI"
    get_var_list_to_array dist_inputs "INPUTS_DISTRIBUTED"

    num_inputs=${#inputs[@]}
    num_mpi_inputs=${#mpi_inputs[@]}
    num_dist_inputs=${#dist_inputs[@]}

    # Set step counts
    current_step=1
    steps_single=3
    steps_mpi=2
    steps_dist=2
    if [[ "${DISABLE_PROVE}" == "1" ]]; then
        steps_single=1
        steps_mpi=0
        steps_dist=0
    fi
    total_steps=$(( 1 + num_inputs * steps_single + num_mpi_inputs * steps_mpi + num_dist_inputs * steps_dist ))

    # Create directories for proof results and logs
    PROOF_RESULTS_DIR="${WORKSPACE_DIR}/proof-results"
    LOGS_DIR="${WORKSPACE_DIR}/logs"
    rm -rf "${PROOF_RESULTS_DIR}" "${LOGS_DIR}"
    mkdir -p "${PROOF_RESULTS_DIR}/single" "${PROOF_RESULTS_DIR}/mpi" "${PROOF_RESULTS_DIR}/distributed"
    mkdir -p "${LOGS_DIR}/single" "${LOGS_DIR}/mpi" "${LOGS_DIR}/distributed"

    # Change to the working directory
    cd "${WORKSPACE_DIR}" || return 1

    # Build mpi command
    MPI_CMD="mpirun --allow-run-as-root --bind-to none -np $MPI_PROCESSES -x OMP_NUM_THREADS=$MPI_THREADS -x RAYON_NUM_THREADS=$MPI_THREADS"

    # Verify existence of all input files
    verify_files_exist "$INPUTS_PATH" "${inputs[@]}" || return 1
    verify_files_exist "$INPUTS_PATH" "${mpi_inputs[@]}" || return 1
    verify_files_exist "$INPUTS_PATH" "${dist_inputs[@]}" || return 1

    step "Generating ${desc} setup..."
    if [[ "${DISABLE_ROM_SETUP}" == "1" ]]; then
        warn "Skipping ROM setup as DISABLE_ROM_SETUP is set to 1"
    else
        rm -rf $HOME/.zisk/cache
        ensure cargo-zisk program-setup -e "${ELF_FILE}" \
        2>&1 | tee romsetup_output.log || return 1
        if ! grep -F "ROM setup successfully completed" romsetup_output.log; then
        err "program setup failed"
        return 1
        fi
    fi

    # -------------------------------------------------------------------------
    # single: cargo-zisk prove (no mpirun)
    # -------------------------------------------------------------------------
    if [ ${num_inputs} -gt 0 ]; then
        for input_file in "${inputs[@]}"; do
            if [[ "${input_file}" != "empty" ]]; then
                input_flag="-i ${INPUTS_PATH}/${input_file}"
            else
                input_flag=""
            fi

            step "Verifying constraints for ${input_file}..."
            ensure cargo-zisk verify-constraints \
                -e "${ELF_FILE}" \
                ${input_flag} \
                2>&1 | tee "constraints_${input_file}.log" || return 1
            if ! grep -F "All global constraints were successfully verified" \
                        "constraints_${input_file}.log"; then
                err "verify constraints failed for ${input_file}"
                return 1
            fi

            if [[ "${DISABLE_PROVE}" != "1" ]]; then
                step "Proving (single) for ${input_file}..."
                rm -rf ${PROOF_DIR}

                ensure cargo-zisk prove \
                    -e "${ELF_FILE}" \
                    ${input_flag} \
                    -o proof.bin $PROVE_FLAGS \
                    2>&1 | tee "prove_${input_file}.log" || return 1
                if ! grep -F "Vadcop Final proof was verified" "prove_${input_file}.log"; then
                    err "prove failed for ${input_file}"
                    return 1
                fi

                # Extract time and cycles from prove log and save to result JSON
                local prove_time
                prove_time=$(sed -nE 's/.*Execution completed in ([0-9.]+)s,.*/\1/p' "prove_${input_file}.log")
                local prove_cycles
                prove_cycles=$(sed -nE 's/.*steps:[[:space:]]*([0-9]+).*/\1/p' "prove_${input_file}.log")
                echo "{\"time\": ${prove_time:-0}, \"cycles\": ${prove_cycles:-0}}" > "${PROOF_RESULTS_DIR}/non-distributed/${input_file}.json"
                result_files+=("${input_file}")

                step "Verifying proof for ${input_file}..."
                ensure cargo-zisk verify \
                    -p ./proof.bin \
                    2>&1 | tee "verify_${input_file}.log" || return 1
                if ! grep -F "STARK proof was verified" "verify_${input_file}.log"; then
                    err "verify proof failed for ${input_file}"
                    return 1
                fi
            fi
        done
    else
        warn "Variable (${inputs_prefix}_SINGLE) is empty or not defined; Skipping single process proving (no mpi)"
    fi

    # -------------------------------------------------------------------------
    # mpi: cargo-zisk prove via mpirun
    # -------------------------------------------------------------------------
    if [ ${num_mpi_inputs} -gt 0 ]; then
        if [[ "${DISABLE_PROVE}" != "1" ]]; then
            for input_file in "${mpi_inputs[@]}"; do
                if [[ "${input_file}" != "empty" ]]; then
                    input_flag="-i ${INPUTS_PATH}/${input_file}"
                else
                    input_flag=""
                fi

                step "Proving (mpi) for ${input_file}..."
                rm -rf ${PROOF_DIR}

                export RAYON_NUM_THREADS=$MPI_THREADS
                ensure $MPI_CMD cargo-zisk prove \
                    -e "${ELF_FILE}" \
                    ${input_flag} \
                    -p 6100 \
                    -o ${PROOF_DIR} $PROVE_FLAGS \
                    2>&1 | tee "${LOGS_DIR}/mpi/prove_mpi_${input_file}.log" || return 1
                if ! grep -qF "Vadcop Final proof was verified" \
                        "${LOGS_DIR}/mpi/prove_mpi_${input_file}.log"; then
                    err "mpi prove failed for ${input_file}"
                    return 1
                fi

                # move result.json into PROOF_RESULTS_DIR (if present)
                if [[ -f "${PROOF_DIR}/result.json" ]]; then
                    mv "${PROOF_DIR}/result.json" "${PROOF_RESULTS_DIR}/mpi/${input_file}.json"
                    result_mpi_files+=("${input_file}")
                fi

                step "Verifying mpi proof for ${input_file}..."
                local verify_proof_file
                verify_proof_file=$(resolve_verify_proof_file) || {
                    err "Verify mpi proof failed for ${input_file}: no proof_*.bin or vadcop_final_proof.bin found in ./proof"
                    return 1
                }

                if ! ensure cargo-zisk verify \
                    -p "${verify_proof_file}" \
                    2>&1 | tee "${LOGS_DIR}/mpi/verify_mpi_${input_file}.log"; then
                    return 1
                fi

                if ! grep -qF "Stark proof was verified" "${LOGS_DIR}/mpi/verify_mpi_${input_file}.log"; then
                    err "Verify mpi proof failed for ${input_file}"
                    return 1
                fi
            done
        fi
    else
        warn "Variable (${inputs_prefix}_MPI) is empty or not defined; Skipping mpi proving"
    fi

    # -------------------------------------------------------------------------
    # distributed: zisk-coordinator + zisk-worker
    # -------------------------------------------------------------------------
    if [ ${num_dist_inputs} -gt 0 ]; then
        if [[ "${DISABLE_PROVE}" != "1" ]]; then
            for input_file in "${dist_inputs[@]}"; do
                if [[ "${input_file}" != "empty" ]]; then
                    input_flag="-i ${INPUTS_PATH}/${input_file}"
                else
                    input_flag=""
                fi

                step "Proving (distributed) for ${input_file}..."
                rm -rf ${PROOF_DIR}

                distributed_prove \
                    "${WORKSPACE_DIR}/${ELF_FILE}" \
                    "${WORKSPACE_DIR}/${INPUTS_PATH}" \
                    "${input_file}" \
                    "${LOGS_DIR}/distributed/prove_dist_${input_file}" || return 1

                local verify_proof_file
                verify_proof_file=$(resolve_verify_proof_file) || {
                    err "distributed prove failed for ${input_file}: no proof_*.bin or vadcop_final_proof.bin found in ./proof"
                    return 1
                }

                # move result.json into PROOF_RESULTS_DIR (if present)
                if [[ -f "${PROOF_DIR}/result.json" ]]; then
                    dest_result_file="${PROOF_RESULTS_DIR}/distributed/${input_file}.json"
                    mv "${PROOF_DIR}/result.json" "${dest_result_file}"
                    result_dist_files+=("${input_file}")
                fi

                step "Verifying distributed proof for ${input_file}..."
                if ! ensure cargo-zisk verify \
                    -p "${verify_proof_file}" \
                    2>&1 | tee "${LOGS_DIR}/distributed/verify_dist_${input_file}.log"; then
                    return 1
                fi

                if ! grep -qF "Stark proof was verified" "${LOGS_DIR}/distributed/verify_dist_${input_file}.log"; then
                    err "Verify distributed proof failed for ${input_file}"
                    return 1
                fi
            done
        fi
    else
        warn "Variable (${inputs_prefix}_DISTRIBUTED) is empty or not defined; skipping distributed proving"
    fi

    rm -rf "${PROOF_DIR}"

    cd ..

    # Print results
    if [ ${num_inputs} -gt 0 ]; then
        echo
        info "Single results:"
        print_proofs_result "${PROOF_RESULTS_DIR}/single" "${result_files[@]}"
    fi
    if [ ${num_mpi_inputs} -gt 0 ]; then
        echo
        info "MPI results:"
        print_proofs_result "${PROOF_RESULTS_DIR}/mpi" "${result_mpi_files[@]}"
    fi
    if [ ${num_dist_inputs} -gt 0 ]; then
        echo
        info "Distributed results:"
        print_proofs_result "${PROOF_RESULTS_DIR}/distributed" "${result_dist_files[@]}"
    fi

    # Clean up result and log files
    rm -rf "${PROOF_RESULTS_DIR}" "${LOGS_DIR}"

    cd "$current_dir"

    success "${desc} have been successfully proved!"
}
