#!/bin/bash

source "./test_elf.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    info "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    cd "${WORKSPACE_DIR}" || return 1

    info "Cloning zisk-eth-client repository..."
    if [[ "$DISABLE_CLONE_REPO" == "1" ]]; then
        warn "Skipping cloning zisk-eth-client repository as DISABLE_CLONE_REPO is set to 1"
    else
        rm -rf zisk-eth-client
        if [[ -n "$ZISK_ETH_CLIENT_BRANCH" ]]; then
            ensure git clone --branch $ZISK_ETH_CLIENT_BRANCH --single-branch https://github.com/0xPolygonHermez/zisk-eth-client.git || return 1
        else
            ensure git clone https://github.com/0xPolygonHermez/zisk-eth-client.git || return 1
        fi
    fi

    ELF_FILE="zisk-eth-client/bin/guests/stateless-validator-reth/elf/zec-reth.elf"
    INPUTS_PATH="zisk-eth-client/bin/guests/stateless-validator-reth/inputs"
    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "BLOCK_INPUTS" "Ethereum blocks" || return 1
}

main
