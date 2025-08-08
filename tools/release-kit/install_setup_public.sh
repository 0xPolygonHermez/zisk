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
        step "Skipping loading .env file since ZISK_GHA is set to 1. Defining SETUP_VERSION from cargo-zisk version"
        
        ZISK_VERSION=$(echo "$(ensure cargo-zisk --version)" | awk '{print $2}')
        info "Installed ZisK version ${ZISK_VERSION}"

        IFS='.' read -r major minor patch <<< "${ZISK_VERSION}"
        SETUP_VERSION="pre-${major}.${minor}.0"
        info "Using SETUP_VERSION: ${SETUP_VERSION}"
    else
        step "Loading environment variables..."
        # Load environment variables from .env file
        load_env || return 1
        confirm_continue || return 1
    fi   

    step  "Downloading public proving key version ${SETUP_VERSION}..."
    ensure curl -L -#o "zisk-provingkey-${SETUP_VERSION}.tar.gz" "https://storage.googleapis.com/zisk-setup/zisk-provingkey-${SETUP_VERSION}.tar.gz" || return 1

    step "Installing public proving key version ${SETUP_VERSION}..."
    rm -rf "$HOME/.zisk/provingKey/"
    ensure tar --overwrite -xf "zisk-provingkey-${SETUP_VERSION}.tar.gz" -C "$HOME/.zisk" || return 1

    step "Generating constant tree files..."
    ensure cargo-zisk check-setup -a || return 1

    step "Deleting downloaded public proving key..."
    rm -rf "zisk-provingkey-${SETUP_VERSION}.tar.gz"

    cd "$current_dir"

    success "Public proving key version ${SETUP_VERSION} installed successfully!"
}

main || return 1