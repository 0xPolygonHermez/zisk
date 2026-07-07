#!/bin/bash

source "./test_elf.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR DISABLE_PROVE ONLY_CPU MPI_PROCESSES MPI_THREADS PROVE_FLAGS || return 1

    PROGRAMS_DIR="$(get_zisk_repo_dir)/test-artifacts/programs"
    ELF_FILE="${PROGRAMS_DIR}/target/elf/riscv64ima-zisk-zkvm-elf/release/diagnostic"

    info "Building diagnostic ELF..."
    cd "${PROGRAMS_DIR}" || return 1

    # cargo-zisk injects the Zisk linker script and preserves config.toml rustflags.
    ensure cargo-zisk build --release -p diagnostic || return 1

    cd "${WORKSPACE_DIR}" || return 1
    DIAGNOSTIC_INPUTS_SINGLE="empty"
    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "DIAGNOSTIC_INPUTS" "ELF Diagnostic" || return 1
}

main
