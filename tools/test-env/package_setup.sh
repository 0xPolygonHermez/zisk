#!/bin/bash

source ./utils.sh

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    total_steps=5

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 0

    cd "$(get_zisk_repo_dir)"

    PROVINGKEY_FILE="zisk-provingkey-${PACKAGE_SETUP_VERSION}.tar.gz"
    VERIFYKEY_FILE="zisk-verifykey-${PACKAGE_SETUP_VERSION}.tar.gz"

    step "Compress proving key..."
    cd build
    ensure tar -czvf "${PROVINGKEY_FILE}" provingKey/ || return 1

    step "Compress verify key..."
    ensure tar -czvf "${VERIFYKEY_FILE}" \
      provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json \
      provingKey/zisk/vadcop_final/vadcop_final.verkey.json \
      provingKey/zisk/vadcop_final/vadcop_final.verifier.bin || return 1

    step "Generate checksums..."
    ensure md5sum "${PROVINGKEY_FILE}" > "${PROVINGKEY_FILE}.md5" || return 1
    ensure md5sum "${VERIFYKEY_FILE}" > "${VERIFYKEY_FILE}.md5" || return 1

    step "Move files to output folder..."
    ensure mv "${PROVINGKEY_FILE}" "${OUTPUT_DIR}" || return 1
    ensure mv "${VERIFYKEY_FILE}" "${OUTPUT_DIR}" || return 1
    ensure mv "${PROVINGKEY_FILE}.md5" "${OUTPUT_DIR}" || return 1
    ensure mv "${VERIFYKEY_FILE}.md5" "${OUTPUT_DIR}" || return 1

    cd "${current_dir}"

    success "ZisK setup packaged successfully!"
}

main
