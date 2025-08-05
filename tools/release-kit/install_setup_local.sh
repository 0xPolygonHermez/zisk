#!/bin/bash

source ./utils.sh
source "$HOME/.bashrc"

OUTPUT_DIR="${HOME}/output"

main() {
    current_step=1
    total_steps=3

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 1

    step "Installing local proving key version ${SETUP_VERSION}..."
    TAR_FILE="${OUTPUT_DIR}/zisk-provingkey-${SETUP_VERSION}.tar.gz"

    if [ ! -f "${TAR_FILE}" ]; then
        err "file '${TAR_FILE}' not found"
        return 1
    fi

    ensure rm -rf "$HOME/.zisk/provingKey/" || return 1
    ensure mkdir -p "$HOME/.zisk" || return 1
    ensure tar --overwrite -xf "${TAR_FILE}" -C "$HOME/.zisk" || return 1

    step "Generating constant tree files..."
    ensure cargo-zisk check-setup -a || return 1
}

main || return 1

