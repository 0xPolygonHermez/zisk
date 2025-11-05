#!/bin/bash

source "./test_elf.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    ELF_FILE="zisk-programs/diagnostic/elf/diagnostic.elf"
    DIAGNOSTIC_INPUTS="empty"
    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "DIAGNOSTIC_INPUTS" "DIAGNOSTIC_DISTRIBUTED_INPUTS" "ELF Diagnostic" || return 1
}

main
