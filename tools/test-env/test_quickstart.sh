#!/bin/bash

source "./utils.sh"

# Runs the quickstart sequence of the hash example (the same one the
# install-from-binaries job runs after installing ZisK from binaries): build,
# run, setup, prove/verify with every backend combination, and run via the SDK.
#
# The whole set of combinations is run first on CPU and then again on GPU
# (adding --gpu), so both proving paths are exercised.
#
# ZisK, its dependencies and the proving keys / setups are expected to be
# already installed.

EXAMPLE_NAME="hash"
GUEST_ELF="target/elf/riscv64ima-zisk-zkvm-elf/release/hash-guest"
GUEST_INPUT="samples/example-input.bin"

EXPECTED_GUEST_OUTPUT="sha256('Hello World!') => 0x7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069"
EXPECTED_SETUP_COMPLETED="Setup completed for /root/zisk/examples/hash/guest/target/elf/riscv64ima-zisk-zkvm-elf/release/hash-guest"
EXPECTED_PROOF_SAVED="INFO: Proof saved to proof.bin"
EXPECTED_STARK_VERIFIED="STARK proof was verified"
EXPECTED_PLONK_VERIFIED="PLONK proof was verified"
EXPECTED_SDK_OUTPUT="sha256('Hello Zisk!') => 0xd3e33ed651e7c8d8d4e30ce9a3e4a40e9f089f22435e279a84ea5f8ae234eead"

# check_output: Fail when the expected fixed string is not present in a log file
#
# Arguments:
#   $1 (log_file) — File to search in
#   $2 (expected) — Fixed string that must appear in it
#   $3 (what)     — Description of the command, used in the error message
check_output() {
    local log_file="$1"
    local expected="$2"
    local what="$3"

    if ! grep -qF "${expected}" "${log_file}"; then
        err "${what} failed: '${expected}' not found in output"
        return 1
    fi
}

# prove_and_verify: Generate a proof of the guest program and verify it, checking
# the expected line in both outputs. Must be run from the guest directory.
#
# Arguments:
#   $1 (label)        — Backend combination label, used in the step title and log names
#   $2 (prove_flags)  — Extra flags for `cargo-zisk prove` (e.g. "--asm --plonk")
#   $3 (verified_str) — Verification line expected from `cargo-zisk verify`
prove_and_verify() {
    local label="$1"
    local prove_flags="$2"
    local verified_str="$3"
    local log_prefix="${LOG_DIR}/prove_${label// /_}"

    step "Proving and verifying guest program [${label}]..."

    # shellcheck disable=SC2086
    ensure cargo-zisk prove --release -i "${GUEST_INPUT}" -o proof.bin ${prove_flags} 2>&1 | tee "${log_prefix}.log" || return 1
    check_output "${log_prefix}.log" "${EXPECTED_PROOF_SAVED}" "prove guest program [${label}]" || return 1

    ensure cargo-zisk verify -p proof.bin 2>&1 | tee "${log_prefix}_verify.log" || return 1
    check_output "${log_prefix}_verify.log" "${verified_str}" "verify proof [${label}]" || return 1
}

# run_via_sdk: Run the host program, which proves and verifies the guest through
# the ZisK SDK. Must be run from the host directory.
#
# Arguments:
#   $1 (label)     — Backend combination label, used in the step title and log name
#   $2 (run_flags) — Runtime flags passed to the host binary (e.g. "--asm")
run_via_sdk() {
    local label="$1"
    local run_flags="$2"
    local log_file="${LOG_DIR}/sdk_${label// /_}.log"

    step "Running guest program via the SDK [${label}]..."

    # shellcheck disable=SC2086
    ensure cargo run --release -- ${run_flags} 2>&1 | tee "${log_file}" || return 1
    check_output "${log_file}" "${EXPECTED_SDK_OUTPUT}" "run via SDK [${label}]" || return 1
}

# run_combinations: Run every prove/verify and SDK combination for one
# accelerator mode — the guest program ones from the guest directory, the host
# SDK ones from the host directory.
#
# Arguments:
#   $1 (label)    — Accelerator label, prefixed to every combination name ("cpu"/"gpu")
#   $2 (gpu_flag) — "--gpu" to prove with GPU acceleration, empty to prove on CPU
#
# Reads has_asm and EXAMPLE_DIR from the calling scope.
run_combinations() {
    local label="$1"
    local gpu_flag="$2"

    cd "${EXAMPLE_DIR}/guest" || return 1

    prove_and_verify "${label} emulator" "${gpu_flag}" "${EXPECTED_STARK_VERIFIED}" || return 1
    if [[ ${has_asm} -eq 1 ]]; then
        prove_and_verify "${label} asm" "--asm ${gpu_flag}" "${EXPECTED_STARK_VERIFIED}" || return 1
    fi
    prove_and_verify "${label} emulator plonk" "--plonk ${gpu_flag}" "${EXPECTED_PLONK_VERIFIED}" || return 1
    if [[ ${has_asm} -eq 1 ]]; then
        prove_and_verify "${label} asm plonk" "--asm --plonk ${gpu_flag}" "${EXPECTED_PLONK_VERIFIED}" || return 1
    fi

    cd "${EXAMPLE_DIR}/host" || return 1

    run_via_sdk "${label} emulator" "${gpu_flag}" || return 1
    if [[ ${has_asm} -eq 1 ]]; then
        run_via_sdk "${label} asm" "--asm ${gpu_flag}" || return 1
    fi
}

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR ONLY_CPU || return 1

    # The ASM backend is Linux-only, so its three combinations (prove, prove with
    # PLONK and run via the SDK) are skipped on macOS.
    local has_asm=1
    if [[ "${PLATFORM}" == "darwin" ]]; then
        has_asm=0
    fi

    # Only enable GPU when not forced to CPU, not on macOS, and the installed
    # cargo-zisk is actually a GPU build (its `--version` description contains
    # "[gpu]", e.g. "cargo-zisk 0.18.0 [gpu] (790f9e2 ...)").
    local has_gpu=0
    if [[ "${ONLY_CPU:-}" != "1" ]] && [[ "${PLATFORM}" != "darwin" ]] && cargo-zisk --version 2>/dev/null | grep -q "\[gpu\]"; then
        has_gpu=1
    fi

    # 4 fixed steps (shared memory, build, run and setup) plus one pass of
    # combinations per accelerator mode: 4 proofs and 2 SDK runs, of which the 3
    # ASM ones are dropped when the ASM backend is unavailable.
    current_step=1
    local steps_per_pass=6
    [[ ${has_asm} -eq 0 ]] && steps_per_pass=3
    total_steps=$(( 4 + steps_per_pass * (1 + has_gpu) ))

    if ! is_gha || [[ "${PLATFORM}" == "linux" ]]; then
        is_proving_key_installed || return 1
    fi

    EXAMPLE_DIR="$(get_zisk_repo_dir)/examples/${EXAMPLE_NAME}"
    if [[ ! -d "${EXAMPLE_DIR}" ]]; then
        err "Example directory not found: ${EXAMPLE_DIR}"
        return 1
    fi

    LOG_DIR="${WORKSPACE_DIR}/quickstart-logs"
    rm -rf "${LOG_DIR}"
    mkdir -p "${LOG_DIR}"

    step "Deleting shared memory..."
    rm -rf /dev/shm/ZISK*
    rm -rf /dev/shm/sem*

    cd "${EXAMPLE_DIR}/guest" || return 1

    step "Building guest program..."
    ensure cargo-zisk build --release 2>&1 | tee "${LOG_DIR}/build.log" || return 1
    # Check the guest ELF has been generated
    if [[ ! -f "${GUEST_ELF}" ]]; then
        err "build guest program failed: ${GUEST_ELF} not found"
        return 1
    fi
    info "${GUEST_ELF} generated"

    step "Running guest program..."
    ensure cargo-zisk run --release -i "${GUEST_INPUT}" 2>&1 | tee "${LOG_DIR}/run.log" || return 1
    check_output "${LOG_DIR}/run.log" "${EXPECTED_GUEST_OUTPUT}" "run guest program" || return 1

    step "Generating guest program setup..."
    ensure cargo-zisk setup --release 2>&1 | tee "${LOG_DIR}/setup.log" || return 1
    check_output "${LOG_DIR}/setup.log" "${EXPECTED_SETUP_COMPLETED}" "guest program setup" || return 1

    if [[ ${has_asm} -eq 0 ]]; then
        warn "Skipping ASM combinations — the ASM backend is not supported on macOS"
    fi
    if [[ ${has_gpu} -eq 0 ]]; then
        warn "Skipping GPU combinations — no GPU build of cargo-zisk is available"
    fi

    # Every combination on CPU first, then the same ones again with --gpu.
    run_combinations "cpu" "" || return 1
    if [[ ${has_gpu} -eq 1 ]]; then
        run_combinations "gpu" "--gpu" || return 1
    fi

    cd "$current_dir"

    success "Quickstart sequence completed successfully!"
}

main
