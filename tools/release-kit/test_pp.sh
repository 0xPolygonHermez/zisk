#!/bin/bash

source "./test_elf.sh"

main() {
    ELF_FILE="pessimistic-proof/elf/pp-keccakf-k256.elf"
    INPUTS_PATH="pessimistic-proof/inputs"
    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "PP_INPUTS" "PP_INPUTS_DISTRIBUTED" "Pessimistic proof" || return 1
}

main || return 1
