#!/bin/bash

source ./utils.sh

main () {
    current_dir=$(pwd)

    current_step=1
    total_steps=5

    # If ZISK_GHA is set to 1, then SETUP_VERSION must be defined
    if [[ "$ZISK_GHA" == "1" ]]; then
        info "Executing install_setup_public.sh script"
        
        # If ZISK_GHA is set, skip loading .env file as env variables are already set from command line
        step "Skipping loading .env file since ZISK_GHA is set to 1"
        
        # Extract ZISK_SETUP_FILE from Cargo.toml
        ZISK_SETUP_FILE=$(grep -oP '(?<=gha_zisk_setup = ")[^"]+' Cargo.toml)

        if [[ -z "$ZISK_SETUP_FILE" ]]; then
            err "ZISK_GHA is set to 1, but ZISK_SETUP_FILE is not defined in Cargo.toml. Aborting"
            return 1
        fi

        info "Using setup file: ${ZISK_SETUP_FILE}"
    else
        step "Loading environment variables..."
        # Load environment variables from .env file
        load_env || return 1
        confirm_continue || return 1

        ZISK_SETUP_FILE="zisk-provingkey-${SETUP_VERSION}.tar.gz"
    fi   

    step  "Downloading public proving key ${ZISK_SETUP_FILE}..."
    ensure curl -L -#o "${ZISK_SETUP_FILE}" "https://storage.googleapis.com/zisk-setup/${ZISK_SETUP_FILE}" || return 1

    step "Installing public proving ${ZISK_SETUP_FILE}..."
    rm -rf "$HOME/.zisk/provingKey/"
    ensure tar --overwrite -xf "${ZISK_SETUP_FILE}" -C "$HOME/.zisk" || return 1

    step "Generating constant tree files..."
    ensure cargo-zisk check-setup -a || return 1

    step "Deleting downloaded public proving key..."
    rm -rf "${ZISK_SETUP_FILE}"

    cd "$current_dir"

    success "Public proving key ${ZISK_SETUP_FILE} installed successfully!"
}

main || return 1