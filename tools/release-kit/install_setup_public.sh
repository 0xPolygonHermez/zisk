#!/bin/bash

source ./utils.sh
source "$HOME/.bashrc"

main () {
    current_step=1
    total_steps=5

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 1

    step  "Downloading public proving key version ${SETUP_VERSION}..."
    ensure curl -L -#o "zisk-provingkey-${SETUP_VERSION}.tar.gz" "https://storage.googleapis.com/zisk-setup/zisk-provingkey-${SETUP_VERSION}.tar.gz" || return 1

    step "Installing public proving key version ${SETUP_VERSION}..."
    rm -rf "$HOME/.zisk/provingKey/"
    ensure tar --overwrite -xf "zisk-provingkey-${SETUP_VERSION}.tar.gz" -C "$HOME/.zisk" || return 1

    step "Generating constant tree files..."
    ensure cargo-zisk check-setup -a || return 1

    step "Deleting downloaded public proving key..."
    rm -rf "zisk-provingkey-${SETUP_VERSION}.tar.gz"
}

main || return 1