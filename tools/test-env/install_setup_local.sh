#!/bin/bash

source ./utils.sh

OUTPUT_DIR="${HOME}/output"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_step=1
    total_steps=3

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 0

    # If ZISK_SETUP_FILE is not set or empty, define it using version from cargo-zisk
    if [[ -z "$ZISK_SETUP_FILE" ]]; then
        ZISK_VERSION=$(echo "$(ensure cargo-zisk --version)" | awk '{print $2}')
        IFS='.' read -r major minor patch <<< "${ZISK_VERSION}"
        ZISK_SETUP_FILE="zisk-provingkey-pre-${major}.${minor}.0.tar.gz"
    fi

    step "Installing local proving key ${ZISK_SETUP_FILE}..."
    TAR_FILE="${OUTPUT_DIR}/${ZISK_SETUP_FILE}"

    if [ ! -f "${TAR_FILE}" ]; then
        err "file '${TAR_FILE}' not found"
        return 1
    fi

    ensure rm -rf "$HOME/.zisk/provingKey/" || return 1
    ensure mkdir -p "$HOME/.zisk" || return 1
    ensure tar --overwrite -xf "${TAR_FILE}" -C "$HOME/.zisk" || return 1

    step "Generating constant tree files..."
    ensure cargo-zisk check-setup -a || return 1

    success "Local proving key ${ZISK_SETUP_FILE} installed successfully!"
}

main

