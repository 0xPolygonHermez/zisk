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
#   print_proofs_result "/home/user/work/proofs/single" file1 file2 file3
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

        # Extract cycles: integer after "cycles":
        local cycles
        cycles=$(sed -nE 's/.*"cycles"[[:space:]]*:[[:space:]]*([0-9]+).*/\1/p' "$fullpath")


        printf "| %-35s | %-10s | %-15s |\n" "$f" "$raw_time" "$cycles"
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

# test_elf: Run proofs for a given ELF program.
#
# Parameters:
#   $1 (elf_file)      – Path to the ELF binary
#   $2 (inputs_path)   – Directory where input files are located
#   $3 (inputs_prefix) – Prefix for input env variables.
#                        The function appends _SINGLE, _MPI to derive
#                        the actual variable names (e.g. BLOCK_INPUTS_SINGLE)
#   $4 (desc)          – Descriptive label for logging
#
# Each proving mode is enabled by populating the corresponding input variable:
#   <PREFIX>_SINGLE      — non-empty → runs "cargo-zisk-dev prove" (no mpirun)
#   <PREFIX>_MPI         — non-empty → runs "cargo-zisk-dev prove" via mpirun
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

    export INPUTS_SINGLE="${!single_var}"
    export INPUTS_MPI="${!mpi_var}"

    declare -a result_files=() result_mpi_files=()
    declare -a inputs=() mpi_inputs=()

    # Load all input arrays; a non-empty list enables that proving mode
    get_var_list_to_array inputs      "INPUTS_SINGLE"
    get_var_list_to_array mpi_inputs  "INPUTS_MPI"

    num_inputs=${#inputs[@]}
    num_mpi_inputs=${#mpi_inputs[@]}

    # Set step counts
    current_step=1
    steps_single=2
    steps_mpi=1
    if [[ "${DISABLE_PROVE}" == "1" ]]; then
        steps_single=1
        steps_mpi=0
    fi
    total_steps=$(( 1 + num_inputs * steps_single + num_mpi_inputs * steps_mpi ))

    # Create directories for proof results and logs
    PROOF_RESULTS_DIR="${WORKSPACE_DIR}/proof-results"
    LOGS_DIR="${WORKSPACE_DIR}/logs"
    rm -rf "${PROOF_RESULTS_DIR}" "${LOGS_DIR}"
    mkdir -p "${PROOF_RESULTS_DIR}/single" "${PROOF_RESULTS_DIR}/mpi"
    mkdir -p "${LOGS_DIR}/single" "${LOGS_DIR}/mpi"

    # Change to the working directory
    cd "${WORKSPACE_DIR}" || return 1

    local gpu_flag=""
    # Only enable GPU when not forced to CPU, not on macOS, and the installed
    # cargo-zisk is actually a GPU build (its `--version` description contains
    # "[gpu]", e.g. "cargo-zisk 0.18.0 [gpu] (790f9e2 ...)").
    if [[ "${ONLY_CPU:-}" != "1" ]] && [[ "${PLATFORM}" != "darwin" ]] && cargo-zisk --version 2>/dev/null | grep -q "\[gpu\]"; then
        gpu_flag="--gpu"
    fi

    # The Rust emulator is the default backend; force the ASM backend (the
    # production path) everywhere except macOS, which has no ASM support.
    local asm_flag=""
    if [[ "${PLATFORM}" != "darwin" ]]; then
        asm_flag="--asm"
    fi

    # Build mpi command
    MPI_CMD="mpirun --allow-run-as-root --bind-to none -np $MPI_PROCESSES -x OMP_NUM_THREADS=$MPI_THREADS -x RAYON_NUM_THREADS=$MPI_THREADS"

    # Verify existence of all input files
    verify_files_exist "$INPUTS_PATH" "${inputs[@]}" || return 1
    verify_files_exist "$INPUTS_PATH" "${mpi_inputs[@]}" || return 1

    # -------------------------------------------------------------------------
    # single: cargo-zisk-dev prove (no mpirun)
    # -------------------------------------------------------------------------
    if [ ${num_inputs} -gt 0 ]; then
        for input_file in "${inputs[@]}"; do
            if [[ "${input_file}" != "empty" ]]; then
                input_flag="-i ${INPUTS_PATH}/${input_file}"
            else
                input_flag=""
            fi

            step "Verifying constraints for ${input_file}..."
            ensure cargo-zisk-dev verify-constraints \
                -e "${ELF_FILE}" \
                ${input_flag} \
                ${asm_flag} \
                ${gpu_flag} \
                2>&1 | tee "${LOGS_DIR}/single/constraints_${input_file}.log" || return 1
            if ! grep -F "All global constraints were successfully verified" \
                        "${LOGS_DIR}/single/constraints_${input_file}.log"; then
                err "verify constraints failed for ${input_file}"
                return 1
            fi

            if [[ "${DISABLE_PROVE}" != "1" ]]; then
                step "Proving (single) for ${input_file}..."
                rm -rf ${PROOF_DIR}

                ensure cargo-zisk-dev prove \
                    -e "${ELF_FILE}" \
                    ${input_flag} \
                    -o proof.bin $PROVE_FLAGS \
                    ${asm_flag} \
                    ${gpu_flag} \
                    2>&1 | tee "${LOGS_DIR}/single/prove_${input_file}.log" || return 1
                if ! grep -F "Vadcop Final proof was verified" "${LOGS_DIR}/single/prove_${input_file}.log"; then
                    err "prove failed for ${input_file}"
                    return 1
                fi

                # Extract time and cycles from prove log and save to result JSON
                local prove_time
                prove_time=$(sed -nE 's/.*Proof Time: ([0-9.]+) seconds.*/\1/p' "${LOGS_DIR}/single/prove_${input_file}.log")
                echo "Extracted proof time: ${prove_time}s"
                local prove_cycles
                prove_cycles=$(sed -nE 's/.*steps:[[:space:]]*([0-9]+).*/\1/p' "${LOGS_DIR}/single/prove_${input_file}.log")
                echo "{\"time\": ${prove_time:-0}, \"cycles\": ${prove_cycles:-0}}" > "${PROOF_RESULTS_DIR}/single/${input_file}.json"
                result_files+=("${input_file}")

                step "Verifying proof for ${input_file}..."
                ensure cargo-zisk verify \
                    -p ./proof.bin \
                    2>&1 | tee "${LOGS_DIR}/single/verify_${input_file}.log" || return 1
                if ! grep -F "STARK proof was verified" "${LOGS_DIR}/single/verify_${input_file}.log"; then
                    err "verify proof failed for ${input_file}"
                    return 1
                fi
            fi
        done
    else
        warn "Variable (${inputs_prefix}_SINGLE) is empty or not defined; Skipping single process proving (no mpi)"
    fi

    # -------------------------------------------------------------------------
    # mpi: cargo-zisk-dev prove via mpirun
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
                ensure $MPI_CMD cargo-zisk-dev prove \
                    -e "${ELF_FILE}" \
                    ${input_flag} \
                    -o ${PROOF_DIR} $PROVE_FLAGS \
                    ${asm_flag} \
                    ${gpu_flag} \
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

                if ! grep -qF "STARK proof was verified" "${LOGS_DIR}/mpi/verify_mpi_${input_file}.log"; then
                    err "Verify mpi proof failed for ${input_file}"
                    return 1
                fi
            done
        fi
    else
        warn "Variable (${inputs_prefix}_MPI) is empty or not defined; Skipping mpi proving"
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

    # Clean up result and log files
    rm -rf "${PROOF_RESULTS_DIR}" "${LOGS_DIR}"

    cd "$current_dir"

    success "${desc} have been successfully proved!"
}
