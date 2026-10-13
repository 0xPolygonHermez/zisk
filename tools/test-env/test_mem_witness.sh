#!/bin/bash
# The device memory witness against the CPU rows, with its fallbacks: proves one Ethereum block
# (the first of BLOCK_INPUTS_SINGLE) with the assembly emulator on the GPU in every configuration
# below. Each proof must verify, and the memory airs' row hashes (ZISK_MEM_TRACE_HASH_DIR) must
# equal those of the CPU path (ZISK_MEM_GPU_FILL=off). The fallback cases must also log the
# fallback they exercise. ZISK_MEM_GPU_FILL is unset for every case but "off" (the device witness
# is the default).

source "./test_elf.sh"

# run_case <name> <expected exit> <log line expected, or ""> <env...>
run_case() {
    local name="$1" want_rc="$2" want_log="$3"
    shift 3
    local hash_dir="${LOGS_DIR}/mem_witness/hash_${name}"
    local log="${LOGS_DIR}/mem_witness/${name}.log"
    rm -rf "${hash_dir}"
    mkdir -p "${hash_dir}"
    info "Case ${name}: $*"
    env -u ZISK_MEM_GPU_FILL "$@" ZISK_MEM_TRACE_HASH_DIR="${hash_dir}" cargo-zisk-dev prove \
        -e "${ZEC_ELF}" -i "${ZEC_INPUTS}/${INPUT_FILE}" -o "${LOGS_DIR}/mem_witness/${name}.bin" \
        --asm --gpu -y $PROVE_FLAGS > "${log}" 2>&1
    local rc=$?
    rm -f "${LOGS_DIR}/mem_witness/${name}.bin"
    if [[ "${rc}" != "${want_rc}" ]]; then
        err "Case ${name}: exit ${rc}, expected ${want_rc} (see ${log})"
        return 1
    fi
    if [[ -n "${want_log}" ]] && ! grep -qF -- "${want_log}" "${log}"; then
        err "Case ${name}: '${want_log}' not logged (see ${log})"
        return 1
    fi
    if [[ "${want_rc}" != "0" ]]; then
        success "Case ${name}: refused as expected"
        return 0
    fi
    if ! grep -qF "proof was verified" "${log}"; then
        err "Case ${name}: the proof did not verify (see ${log})"
        return 1
    fi
    if [[ "${name}" == "off" ]]; then
        REF_DIR="${hash_dir}"
        success "Case ${name}: $(ls "${hash_dir}" | wc -l) instance hashes taken as the reference"
        return 0
    fi
    local f n=0
    for f in "${REF_DIR}"/*.hash; do
        n=$((n + 1))
        if ! cmp -s "${f}" "${hash_dir}/$(basename "${f}")"; then
            err "Case ${name}: rows of $(basename "${f}" .hash) differ from the CPU rows"
            return 1
        fi
    done
    success "Case ${name}: ${n} instances identical to the CPU rows"
}

main() {
    info "▶️  Running $(basename "$0") script..."

    load_env BLOCK_INPUTS_SINGLE PROVE_FLAGS ZEC_GUEST || return 1
    resolve_zec_guest || return 1
    zec_guest_inputs BLOCK_INPUTS_SINGLE || return 1
    is_proving_key_installed || return 1

    if ! cargo-zisk --version 2>/dev/null | grep -q "\[gpu\]"; then
        err "cargo-zisk is not a GPU build; the device memory witness needs one"
        return 1
    fi

    local -a inputs=()
    get_var_list_to_array inputs BLOCK_INPUTS_SINGLE
    if (( ${#inputs[@]} == 0 )); then
        err "BLOCK_INPUTS_SINGLE is empty"
        return 1
    fi
    INPUT_FILE="${inputs[0]}"
    verify_files_exist "${ZEC_INPUTS}" "${INPUT_FILE}" || return 1

    LOGS_DIR="${WORKSPACE_DIR}/logs"
    mkdir -p "${LOGS_DIR}/mem_witness"
    cd "${WORKSPACE_DIR}" || return 1

    local plan_fail="no device plan for this block; planning it on the CPU"
    # The CPU path: its hashes are the reference.
    run_case off 0 "" ZISK_MEM_GPU_FILL=off || return 1
    # The device witness (the default).
    run_case arena 0 "" || return 1
    # One owned instance without a staged image: its rows are built on demand.
    run_case unstaged 0 "" ZISK_MEM_TEST_UNSTAGED=0.1 || return 1
    # Pieces of fewer potentials: block records get split into sub-records.
    run_case split_records 0 "" ZISK_MEM_TEST_PIECE_POTENTIALS=65536 || return 1
    # A planner fault: the CPU planner replays the chunks, the CPU witness follows.
    run_case plan_fail 0 "${plan_fail}" ZISK_MEM_TEST_PLAN_FAIL=1 || return 1
    # The light memory-ops stream has no values: the device witness is refused before proving.
    run_case light_refused 1 "the device witness needs them" ZISK_MOPS_LIGHT=1 || return 1

    success "Device memory witness: every configuration proved, verified and matched the CPU rows"
}

main
