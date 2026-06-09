#!/bin/bash

source "./test_elf.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    info "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    cd "${WORKSPACE_DIR}" || return 1

    ELF_FILE="zisk-eth-client/bin/guests/stateless-validator-reth/target/elf/riscv64ima-zisk-zkvm-elf/release/zec-reth"
    INPUTS_PATH="zisk-eth-client/bin/guests/stateless-validator-reth/inputs"

    info "Verifying zec-reth ELF exists..."
    if [[ ! -f "${ELF_FILE}" ]]; then
        err "zec-reth ELF not found: ${ELF_FILE}. Please run build_zec_reth.sh first."
        return 1
    fi

    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "BLOCK_INPUTS" "Ethereum blocks" || return 1
}

main
