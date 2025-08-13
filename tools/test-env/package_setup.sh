#!/bin/bash

source ./utils.sh

OUTPUT_DIR="${HOME}/output"

main() {
    current_step=1
    total_steps=5

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 0

    mkdir -p "${WORKSPACE_DIR}"
    cd "${WORKSPACE_DIR}"

    PROVINGKEY_FILE="zisk-provingkey-${PACKAGE_SETUP_VERSION}.tar.gz"
    VERIFYKEY_FILE="zisk-verifykey-${PACKAGE_SETUP_VERSION}.tar.gz"

    step "Compress proving key..."
    cd zisk/build
    ensure tar -czvf "${PROVINGKEY_FILE}" provingKey/ || return 1

    step "Compress verify key..."
    ensure tar -czvf "${VERIFYKEY_FILE}" \
      provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json \
      provingKey/zisk/vadcop_final/vadcop_final.verkey.json \
      provingKey/zisk/vadcop_final/vadcop_final.verifier.bin || return 1

    step "Generate checksums..."
    ensure md5sum "${PROVINGKEY_FILE}" > "${PROVINGKEY_FILE}.md5" || return 1
    ensure md5sum "${VERIFYKEY_FILE}" > "${VERIFYKEY_FILE}.md5" || return 1

    cd ../..

    step "Move files to output folder..."
    ensure mv "${DEFAULT_ZISK_REPO_DIR}/build/${PROVINGKEY_FILE}" "${OUTPUT_DIR}" || return 1
    ensure mv "${DEFAULT_ZISK_REPO_DIR}/build/${VERIFYKEY_FILE}" "${OUTPUT_DIR}" || return 1
    ensure mv "${DEFAULT_ZISK_REPO_DIR}/build/${PROVINGKEY_FILE}.md5" "${OUTPUT_DIR}" || return 1
    ensure mv "${DEFAULT_ZISK_REPO_DIR}/build/${VERIFYKEY_FILE}.md5" "${OUTPUT_DIR}" || return 1
}

main
