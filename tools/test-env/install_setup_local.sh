#!/bin/bash

source ./utils.sh

OUTPUT_DIR="${HOME}/output"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_step=1
    total_steps=1

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR ZISK_SETUP_FILE || return 1

    # If ZISK_SETUP_FILE is not set or empty, define it using setup_version from setup/Cargo.toml
    if [[ -z "$ZISK_SETUP_FILE" ]]; then
        SETUP_VERSION="$(sed -nE 's/^setup_version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$(get_zisk_repo_dir)/setup/Cargo.toml")"
        [[ -n "$SETUP_VERSION" ]] || { err "could not read setup_version from setup/Cargo.toml"; return 1; }
        ZISK_SETUP_FILE="zisk-provingkey-${SETUP_VERSION}.tar.gz"
    fi

    step "Installing local proving key ${ZISK_SETUP_FILE}..."
    TAR_FILE="${OUTPUT_DIR}/${ZISK_SETUP_FILE}"

    if [ ! -f "${TAR_FILE}" ]; then
        err "file '${TAR_FILE}' not found"
        return 1
    fi

    ensure mkdir -p "$HOME/.zisk" || return 1
    ensure rm -rf "$HOME/.zisk/provingKey/" || return 1
    ensure tar -xf "${TAR_FILE}" -C "$HOME/.zisk" || return 1

    success "Local proving key ${ZISK_SETUP_FILE} installed successfully!"
}

main
