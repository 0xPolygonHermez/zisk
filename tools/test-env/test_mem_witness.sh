#!/bin/bash
# The device memory witness against the CPU rows, with its fallbacks: proves one Ethereum block
# (the first of BLOCK_INPUTS_SINGLE) with the assembly emulator on the GPU in every configuration
# below, then the diagnostic program (every precompile) on both paths. Each proof must verify, and
# the memory airs' row hashes (ZISK_MEM_TRACE_HASH_DIR) must equal those of the CPU path
# (ZISK_MEM_GPU_FILL=0). The fallback cases must also log the fallback they exercise.
# ZISK_MEM_GPU_FILL is unset for every case but the "off" ones (ZISK_MEM_GPU_FILL=0, the device
# witness is the default).
#
# Needs cargo-zisk built with `--features mem-witness-test`: the knobs below and the row-hash dump
# exist in that build only. The planner of such a build logs "[mops] test knobs enabled" at its
# setup; the first device case checks for that line and fails otherwise.
#
# Fallbacks, each forced by a test knob at the point the planner detects the real condition:
#   ZISK_MEM_TEST_PLAN_FAIL        the device has no plan: the CPU planner replays the chunks
#   ZISK_MEM_TEST_CHUNK_REJECT=<k> the planner rejects chunk k: same
#   ZISK_MEM_TEST_RETENTION=<n>    the block exceeds the retention cap: GPU plan, CPU witness
#   ZISK_MEM_TEST_LIGHT_RECORD=1   a light-form record in the stream: same
#   ZISK_MEM_TEST_PREP_FAIL=<step> a step of the preparation (ram, rom, input, align) has no room:
#                                  the preparation fails and the first memory instance sends the
#                                  block's memory instances to the CPU, collected in one pass
#   ZISK_MEM_TEST_FILL_FAIL=<f>.<i> an instance the device cannot fill: it and the rest of the
#                                  block's memory instances are collected on the CPU in one pass
#   ZISK_MEM_TEST_SCRATCH_OVERFLOW=1 a fill retried without its image, then with the images dropped
#   ZISK_MEM_TEST_UNSTAGED=<f>.<i> an owned instance without an image: built on demand
#   ZISK_MEM_TEST_PIECE_POTENTIALS=<n> smaller pieces: block records split into sub-records

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
        -e "${CASE_ELF}" ${CASE_INPUT} -o "${LOGS_DIR}/mem_witness/${name}.bin" \
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
    if [[ "${name}" == *off ]]; then
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

    load_env ZISK_REPO_DIR BLOCK_INPUTS_SINGLE PROVE_FLAGS ZEC_GUEST || return 1
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

    CASE_ELF="${ZEC_ELF}"
    CASE_INPUT="-i ${ZEC_INPUTS}/${INPUT_FILE}"
    local plan_fail="no device plan for this block; planning it on the CPU"
    # The CPU path: its hashes are the reference.
    run_case off 0 "" ZISK_MEM_GPU_FILL=0 || return 1
    # The device witness (the default), with the build checked for the test knobs.
    run_case arena 0 "" || return 1
    if ! grep -qF "[mops] test knobs enabled" "${LOGS_DIR}/mem_witness/arena.log"; then
        err "build cargo-zisk with --features mem-witness-test (the test knobs and the row-hash dump are off)"
        return 1
    fi
    # One owned instance without a staged image: its rows are built on demand.
    run_case unstaged 0 "" ZISK_MEM_TEST_UNSTAGED=0.1 || return 1
    # Pieces of fewer potentials: block records get split into sub-records.
    run_case split_records 0 "" ZISK_MEM_TEST_PIECE_POTENTIALS=65536 || return 1
    # A planner fault: the CPU planner replays the chunks, the CPU witness follows.
    run_case plan_fail 0 "${plan_fail}" ZISK_MEM_TEST_PLAN_FAIL=1 || return 1
    # A chunk the planner rejects: the block is planned on the CPU.
    run_case chunk_reject 0 "a chunk was rejected (see above); no device plan for this block" ZISK_MEM_TEST_CHUNK_REJECT=3 || return 1
    # The block exceeds the retention cap: the GPU plan stands, the CPU witness follows.
    # The ops pool's reservation meets the retained accesses: the retention is dropped, the retain kernels
    # are waited out, the pool grows on; the GPU plan stands, the witness is on the CPU.
    run_case pool_cross 0 "the ops pool meets the retained accesses" ZISK_MEM_TEST_POOL_CROSS=3 || return 1
    run_case retention 0 "the retained accesses are incomplete for this block" ZISK_MEM_TEST_RETENTION=1000000 || return 1
    # A light-form record seen in the stream: the GPU plan stands, no device witness.
    run_case light_record 0 "the stream is in the light form" ZISK_MEM_TEST_LIGHT_RECORD=1 || return 1
    # A step of the preparation without room: the preparation fails, the block's memory instances go to the CPU.
    run_case prep_fail_ram 0 "the arrival index can address; RAM witness off" ZISK_MEM_TEST_PREP_FAIL=ram || return 1
    run_case prep_fail_rom 0 "no room for the access table below the retained accesses" ZISK_MEM_TEST_PREP_FAIL=rom || return 1
    run_case prep_fail_input 0 "input image below the retained accesses" ZISK_MEM_TEST_PREP_FAIL=input || return 1
    run_case prep_fail_align 0 "no room for the access order below the retained accesses" ZISK_MEM_TEST_PREP_FAIL=align || return 1
    # An instance the device cannot fill: it and the remaining memory instances are collected on the CPU in one pass.
    local fill_fail="memory witness on the device failed"
    run_case fill_fail_ram 0 "${fill_fail}" ZISK_MEM_TEST_FILL_FAIL=0.1 || return 1
    run_case fill_fail_rom 0 "${fill_fail}" ZISK_MEM_TEST_FILL_FAIL=1.0 || return 1
    run_case fill_fail_input 0 "${fill_fail}" ZISK_MEM_TEST_FILL_FAIL=2.0 || return 1
    run_case fill_fail_align 0 "${fill_fail}" ZISK_MEM_TEST_FILL_FAIL=3.1 || return 1
    # A fill whose scratch overflows beside the staged images: retried without its image, then with every image dropped.
    run_case scratch_overflow 0 "staged images dropped, the fills need the room" ZISK_MEM_TEST_SCRATCH_OVERFLOW=1 || return 1
    # The MemAlign region too small for the block's unaligned accesses (a 16 MB region instead of 2560):
    # the overflow is detected and the block's memory instances go to the CPU. No test build needed.
    run_case align_region_overflow 0 "overflowed; witness off" ZISK_MEM_ALIGN_MB=16 || return 1

    # The diagnostic program: every precompile's records in the stream, which a block does not cover.
    CASE_ELF="$(get_zisk_repo_dir)/test-artifacts/programs/target/elf/riscv64ima-zisk-zkvm-elf/release/diagnostic"
    CASE_INPUT=""
    if [[ ! -f "${CASE_ELF}" ]]; then
        err "diagnostic ELF not built (menu option 11 builds it)"
        return 1
    fi
    run_case diagnostic_off 0 "" ZISK_MEM_GPU_FILL=0 || return 1
    run_case diagnostic 0 "" || return 1

    success "Device memory witness: every configuration proved, verified and matched the CPU rows"
}

main
