#!/bin/bash

source "./test_elf.sh"

main() {
    ELF_FILE="eth-client/elf/zec-keccakf-k256-sha2-bn254.elf"
    INPUTS_PATH="eth-client/inputs"
    test_elf "${ELF_FILE}" "${INPUTS_PATH}" "BLOCK_INPUTS" "BLOCK_INPUTS_DISTRIBUTED" "Ethereum blocks" || return 1

    # DIR="./path"

    # if [[ ! -d "$DIR" ]]; then
    # echo "Directory '$DIR' does not exist"
    # exit 1
    # fi

    # BLOCK_FOLDER_INPUTS=""

    # for file in "$DIR"/*; do
    # if [[ -f "$file" ]]; then
    #     filename=$(basename "$file")
    #     BLOCK_FOLDER_INPUTS+="$filename:"
    # fi
    # done

    # BLOCK_FOLDER_INPUTS=${BLOCK_FOLDER_INPUTS%:}

    # export BLOCK_FOLDER_INPUTS

    # echo "BLOCK_FOLDER_INPUTS=$BLOCK_FOLDER_INPUTS"    
}

main || return 1
