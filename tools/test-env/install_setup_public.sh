#!/bin/bash

source ./utils.sh

main () {
    info "▶️  Running $(basename "$0") script..."

    current_step=1
    total_steps=5

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1
    confirm_continue || return 0

    # If ZISK_SETUP_FILE is not set or empty, define it using version from cargo-zisk
    if [[ -z "$ZISK_SETUP_FILE" ]]; then
        ZISK_VERSION=$(echo "$(ensure cargo-zisk --version)" | awk '{print $2}')
        IFS='.' read -r major minor patch <<< "${ZISK_VERSION}"
        ZISK_SETUP_FILE="zisk-provingkey-pre-${major}.${minor}.0.tar.gz"
    fi

    info "Using setup file: ${ZISK_SETUP_FILE}"

    step  "Downloading public proving key ${ZISK_SETUP_FILE}..."
    ensure curl -L -#o "${ZISK_SETUP_FILE}" "https://storage.googleapis.com/zisk-setup/${ZISK_SETUP_FILE}" || return 1

    step "Installing public proving ${ZISK_SETUP_FILE}..."
    rm -rf "$HOME/.zisk/provingKey/"
    ensure tar --overwrite -xf "${ZISK_SETUP_FILE}" -C "$HOME/.zisk" || return 1

    step "Generating constant tree files..."
    ensure cargo-zisk check-setup -a || return 1

    step "Deleting downloaded public proving key..."
    rm -rf "${ZISK_SETUP_FILE}"

    success "Public proving key ${ZISK_SETUP_FILE} installed successfully!"
}

main