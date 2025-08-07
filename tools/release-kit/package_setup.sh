#!/bin/bash

source ./utils.sh

OUTPUT_DIR="${HOME}/output"

main() {
    current_step=1
    total_steps=5

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 1

    mkdir -p "${HOME}/work"
    cd "${HOME}/work"

    step "Compress proving key..."
    cd zisk/build
    ensure tar -czvf "zisk-provingkey-${SETUP_VERSION}.tar.gz" provingKey/ || return 1

    step "Compress verify key..."
    ensure tar -czvf "zisk-verifykey-${SETUP_VERSION}.tar.gz" \
      provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json \
      provingKey/zisk/vadcop_final/vadcop_final.verkey.json \
      provingKey/zisk/vadcop_final/vadcop_final.verifier.bin || return 1

    step "Generate checksums..."
    ensure md5sum "zisk-provingkey-${SETUP_VERSION}.tar.gz" > "zisk-provingkey-${SETUP_VERSION}.tar.gz.md5" || return 1
    ensure md5sum "zisk-verifykey-${SETUP_VERSION}.tar.gz" > "zisk-verifykey-${SETUP_VERSION}.tar.gz.md5" || return 1

    cd ../..

    step "Move files to output folder..."
    ensure mv "${HOME}/work/zisk/build/zisk-provingkey-${SETUP_VERSION}.tar.gz" "${OUTPUT_DIR}" || return 1
    ensure mv "${HOME}/work/zisk/build/zisk-verifykey-${SETUP_VERSION}.tar.gz" "${OUTPUT_DIR}" || return 1
    ensure mv "${HOME}/work/zisk/build/zisk-provingkey-${SETUP_VERSION}.tar.gz.md5" "${OUTPUT_DIR}" || return 1
    ensure mv "${HOME}/work/zisk/build/zisk-verifykey-${SETUP_VERSION}.tar.gz.md5" "${OUTPUT_DIR}" || return 1
}

main || return 1
