#!/bin/bash

source "./test_elf.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    info "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    cd "${WORKSPACE_DIR}" || return 1

    info "Cloning zisk-eth-client repository..."
    if [[ -n "$ZISK_ETH_CLIENT_BRANCH" ]]; then
        if [[ "$DISABLE_CLONE_REPO" == "1" ]]; then
            warn "Skipping cloning zisk-eth-client repository as DISABLE_CLONE_REPO is set to 1"
        else
            rm -rf zisk-eth-client
            ensure git clone --branch $ZISK_ETH_CLIENT_BRANCH --single-branch https://github.com/0xPolygonHermez/zisk-eth-client.git || return 1
        fi
    else
        info "Skipping cloning zisk-eth-client repository as ZISK_ETH_CLIENT_BRANCH is not defined"
    fi

    ELF_FILE="zisk-eth-client/bin/guests/stateless-validator-reth/elf/zec-reth.elf"
    INPUTS_PATH="zisk-eth-client/bin/guests/stateless-validator-reth/inputs"
    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "BLOCK_INPUTS" "Ethereum blocks" || return 1
}

main
