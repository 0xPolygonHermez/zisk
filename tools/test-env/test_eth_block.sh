#!/bin/bash

source "./test_elf.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env BLOCK_INPUTS_SINGLE BLOCK_INPUTS_MPI DISABLE_PROVE ONLY_CPU MPI_PROCESSES MPI_THREADS PROVE_FLAGS ZEC_GUEST || return 1

    resolve_zec_guest || return 1

    cd "${WORKSPACE_DIR}" || return 1

    info "Verifying zec-${ZEC_GUEST} ELF exists..."
    if [[ ! -f "${ZEC_ELF}" ]]; then
        err "zec-${ZEC_GUEST} ELF not found: ${ZEC_ELF}. Please run build_zec_guest.sh first."
        return 1
    fi

    zec_guest_inputs BLOCK_INPUTS_SINGLE || return 1
    zec_guest_inputs BLOCK_INPUTS_MPI || return 1

    test_elf "${ZEC_ELF}" "${ZEC_INPUTS}" "BLOCK_INPUTS" "Ethereum blocks" || return 1
}

main
