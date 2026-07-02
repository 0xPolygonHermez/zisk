#!/bin/bash

source "./test_elf.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    info "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    PROGRAMS_DIR="$(get_zisk_repo_dir)/test-artifacts/programs"
    ELF_FILE="${PROGRAMS_DIR}/target/elf/riscv64ima-zisk-zkvm-elf/release/diagnostic"
    ZISK_LINKER_SCRIPT="$(get_zisk_repo_dir)/ziskbuild/zisk_linker_script.ld"

    info "Building diagnostic ELF..."
    cd "${PROGRAMS_DIR}" || return 1
    
    # Append to any RUSTFLAGS already set in the environment instead of replacing
    # them. `${RUSTFLAGS:+$RUSTFLAGS }` expands to the existing flags plus a space
    # only when RUSTFLAGS is non-empty, so there is no stray leading space.
    ensure env CARGO_TARGET_DIR="${PROGRAMS_DIR}/target/elf" \
           env RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-T${ZISK_LINKER_SCRIPT}" \
        cargo +zisk build --release  -p diagnostic --target riscv64ima-zisk-zkvm-elf || return 1

    cd "${WORKSPACE_DIR}" || return 1
    DIAGNOSTIC_INPUTS_SINGLE="empty"
    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "DIAGNOSTIC_INPUTS" "ELF Diagnostic" || return 1
}

main
