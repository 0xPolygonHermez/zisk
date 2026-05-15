#!/bin/bash

source "./test_elf.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    info "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    cd "${WORKSPACE_DIR}" || return 1

    info "Cloning zisk-testvectors repository..."
    if [[ "$DISABLE_CLONE_REPO" == "1" ]]; then
        warn "Skipping cloning zisk-testvectors repository as DISABLE_CLONE_REPO is set to 1"
    else
        rm -rf zisk-testvectors
        if [[ -n "$ZISK_TESTVECTORS_BRANCH" ]]; then
            ensure git clone --branch "$ZISK_TESTVECTORS_BRANCH" --depth 1 --single-branch https://github.com/0xPolygonHermez/zisk-testvectors.git || return 1
        else
            ensure git clone --depth 1 --single-branch https://github.com/0xPolygonHermez/zisk-testvectors.git || return 1
        fi
    fi

    ELF_FILE="zisk-testvectors/zisk-programs/diagnostic/elf/diagnostic.elf"
    DIAGNOSTIC_INPUTS_SINGLE="empty"
    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "DIAGNOSTIC_INPUTS" "ELF Diagnostic" || return 1
}

main
