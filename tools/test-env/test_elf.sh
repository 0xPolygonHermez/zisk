#!/bin/bash

source "./utils.sh"

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
    printf "| %-30s | %-10s | %-15s |\n" "------------------------------" "----------" "---------------"
    printf "| %-30s | %-10s | %-15s |\n" "File"                           "Time (s)"   "Cycles"
    printf "| %-30s | %-10s | %-15s |\n" "------------------------------" "----------" "---------------"

    for f in "${files[@]}"; do
        local fullpath="${base_path}/${f}.json"

        if [[ ! -f "$fullpath" ]]; then
            printf "| %-30s | %-10s | %-15s |\n" "$f" "N/A" "N/A"
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


        printf "| %-30s | %-10s | %-15s |\n" "$f" "$time_int" "$cycles"
    done

    printf "| %-30s | %-10s | %-15s |\n" "------------------------------" "----------" "---------------"

    echo
}

# delete_proofs_result: Remove proof result JSON files after processing
#
# Arguments:
#   $1 (base_path) — Directory path where result JSON files are stored
#   $2…$n (files) — Input filenames (without “.json”) of the result files to delete
#
# Example:
#   delete_proofs_result "/home/user/work/proofs/distributed" file1 file2 file3
delete_proofs_result() {
    local base_path="$1"
    shift
    local files=("$@")

    for f in "${files[@]}"; do
        rm -f "${base_path}/${f}.json"
    done
}

# test_elf: Run proofs for a given ELF program with both non-distributed and distributed inputs
#
# Parameters:
#   $1 (elf_file)              – Path to the ELF binary
#   $2 (inputs_path)           – Directory where input files are located
#   $3 (inputs_var_name)       – Name of the env variable holding comma-separated non-distributed input filenames
#   $4 (dist_inputs_var_name)  – Name of the env variable holding comma-separated distributed input filenames
#   $5 (desc)                  – Descriptive label for logging
#
# Example:
#  prove "program.elf" "inputs" "INPUTS" "INPUTS_DISTRIBUTED" "Proving program"
test_elf() {
    local elf_file="$1"
    local inputs_path="$2"
    local inputs_var_name="$3"
    local dist_inputs_var_name="$4"
    local desc="$5"

    current_dir=$(pwd)

    info "Executing ${desc} script"

    if [[ "${PLATFORM}" == "linux" ]]; then
        is_proving_key_installed || return 1
    fi

    info "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1
    confirm_continue || return 0

    export ELF_FILE="$elf_file"
    export INPUTS_PATH="$inputs_path"
    export INPUTS="${!inputs_var_name}"
    export INPUTS_DISTRIBUTED="${!dist_inputs_var_name}"

    declare -a result_files=() result_dist_files=()
    declare -a inputs=() dist_inputs=()

    # Get list of input files
    get_var_list_to_array inputs "INPUTS"
    get_var_list_to_array dist_inputs "INPUTS_DISTRIBUTED"

    num_inputs=${#inputs[@]}
    num_dist_inputs=${#dist_inputs[@]}

    # Set step counts
    current_step=1
    steps_no_dist=3
    steps_dist=1
    if [[ "${DISABLE_PROVE}" != "1" ]]; then
        steps_no_dist=1
        steps_dist=0
    fi
    total_steps=$(( 2 + num_inputs * $steps_no_dist + num_dist_inputs * $steps_dist ))

    # Create directories for proof results
    PROOF_RESULTS_DIR="${WORKSPACE_DIR}/proof-results"
    rm -rf "${PROOF_RESULTS_DIR}"
    mkdir -p "${PROOF_RESULTS_DIR}"
    mkdir -p "${PROOF_RESULTS_DIR}/non-distributed"
    mkdir -p "${PROOF_RESULTS_DIR}/distributed"

    # Change to the working directory
    cd "${WORKSPACE_DIR}" || return 1

    # Build mpi command
    MPI_CMD="mpirun --allow-run-as-root --bind-to none -np $DISTRIBUTED_PROCESSES -x OMP_NUM_THREADS=$DISTRIBUTED_THREADS -x RAYON_NUM_THREADS=$DISTRIBUTED_THREADS"

    step "Cloning zisk-testvectors repository..."
    if [[ -n "$ZISK_TESTVECTORS_BRANCH" ]]; then
        if [[ "$DISABLE_CLONE_REPO" == "1" ]]; then
            warn "Skipping cloning zisk-testvectors repository as DISABLE_CLONE_REPO is set to 1"
        else
            rm -rf zisk-testvectors
            ensure git clone https://github.com/0xPolygonHermez/zisk-testvectors.git || return 1
            cd zisk-testvectors
            ensure git checkout "$ZISK_TESTVECTORS_BRANCH" || return 1
            cd ..
        fi
    else
        info "Skipping cloning zisk-testvectors repository as ZISK_TESTVECTORS_BRANCH is not defined"
    fi
    cd zisk-testvectors || return 1

    # Verify existence of all input files
    verify_files_exist "$INPUTS_PATH" "${inputs[@]}" || return 1
    verify_files_exist "$INPUTS_PATH" "${dist_inputs[@]}" || return 1

    step "Generating ${desc} setup..."
    if [[ "${DISABLE_ROM_SETUP}" == "1" ]]; then
        warn "Skipping ROM setup as DISABLE_ROM_SETUP is set to 1"
    else
        rm -rf $HOME/.zisk/cache
        ensure cargo-zisk rom-setup -e "${ELF_FILE}" \
        2>&1 | tee romsetup_output.log || return 1
        if ! grep -F "ROM setup successfully completed" romsetup_output.log; then
        err "program setup failed"
        return 1
        fi
    fi

    # Process inputs in non-distributed
    if [ ${num_inputs} -gt 0 ]; then
        for input_file in "${inputs[@]}"; do
            step "Verifying constraints for ${input_file}..."
            if [[ "${BUILD_GPU}" == "1" ]]; then
                warn "Skipping verify constraints for GPU mode"
            else
                ensure cargo-zisk verify-constraints \
                    -e "${ELF_FILE}" \
                    -i "${INPUTS_PATH}/${input_file}" \
                    2>&1 | tee "constraints_${input_file}.log" || return 1
                if ! grep -F "All global constraints were successfully verified" \
                         "constraints_${input_file}.log"; then
                    err "verify constraints failed for ${input_file}"
                    return 1
                fi
            fi

            if [[ "${DISABLE_PROVE}" != "1" ]]; then
                step "Proving (non-distributed) for ${input_file}..."
                ensure cargo-zisk prove \
                    -e "${ELF_FILE}" \
                    -i "${INPUTS_PATH}/${input_file}" \
                    -o proof $PROVE_FLAGS \
                    2>&1 | tee "prove_${input_file}.log" || return 1
                if ! grep -F "Vadcop Final proof was verified" "prove_${input_file}.log"; then
                    err "prove failed for ${input_file}"
                    return 1
                fi

                # move result.json into PROOF_RESULTS_DIR
                mv proof/result.json "${PROOF_RESULTS_DIR}/non-distributed/${input_file}.json"
                result_files+=("${input_file}")

                step "Verifying proof for ${input_file}..."
                ensure cargo-zisk verify \
                    -p ./proof/vadcop_final_proof.bin \
                    2>&1 | tee "verify_${input_file}.log" || return 1
                if ! grep -F "Stark proof was verified" "verify_${input_file}.log"; then
                    err "verify proof failed for ${input_file}"
                    return 1
                fi
            fi
        done
    else
        warn "non-distributed inputs variable is empty or not defined; skipping non-distributed proofs"
    fi

    # Process inputs in distributed mode
    if [ ${num_dist_inputs} -gt 0 ]; then
        if [[ "${DISABLE_PROVE}" != "1" ]]; then
            for input_file in "${dist_inputs[@]}"; do
                step "Proving (distributed) for ${input_file}..."
                export RAYON_NUM_THREADS=$DISTRIBUTED_THREADS
                ensure $MPI_CMD cargo-zisk prove \
                    -e "${ELF_FILE}" \
                    -i "${INPUTS_PATH}/${input_file}" \
                    -o proof $PROVE_FLAGS \
                    2>&1 | tee "prove_dist_${input_file}.log" || return 1
                if ! grep -F "Vadcop Final proof was verified" \
                        "prove_dist_${input_file}.log"; then
                    err "distributed prove failed for ${input_file}"
                    return 1
                fi

                # move result.json into PROOF_RESULTS_DIR
                dest_result_file="${PROOF_RESULTS_DIR}/distributed/${input_file}.json"
                mv proof/result.json "${dest_result_file}"
                result_dist_files+=("${input_file}")
            done
        fi
    else
        warn "distributed inputs variable is empty or not defined; skipping distributed proofs"
    fi

    cd ..

    # Print results
    if [ ${num_inputs} -gt 0 ]; then
        echo
        info "Non-distributed results:"
        print_proofs_result "${PROOF_RESULTS_DIR}/non-distributed" "${result_files[@]}"
    fi
    if [ ${num_dist_inputs} -gt 0 ]; then
        echo
        info "Distributed results:"
        print_proofs_result "${PROOF_RESULTS_DIR}/distributed" "${result_dist_files[@]}"
    fi

    # Clean up result files
    delete_proofs_result "${PROOF_RESULTS_DIR}/non-distributed" "${result_files[@]}"
    delete_proofs_result "${PROOF_RESULTS_DIR}/distributed" "${result_dist_files[@]}"
    rm -rf "${PROOF_RESULTS_DIR}"

    cd "$current_dir"

    success "${desc} have been successfully proved!"
}
