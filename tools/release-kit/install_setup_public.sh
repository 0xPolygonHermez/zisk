#!/bin/bash

source ./utils.sh

main () {
    current_dir=$(pwd)

    current_step=1
    total_steps=5

    info "Executing install_setup_public.sh script"

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1
    confirm_continue || return 1

    # If ZISK_GHA is set to 1 we get the setup file from the Cargo.toml
    if [[ "$ZISK_GHA" == "1" ]]; then        
        # If ZISK_REPO_DIR is not set, use default
        if [[ -z "${ZISK_REPO_DIR}" ]]; then
            ZISK_REPO_DIR="${DEFAULT_ZISK_REPO_DIR}"
        fi

        # Get the setup file from the Cargo.toml
        ZISK_SETUP_FILE=$(get_var_from_cargo_toml "${ZISK_REPO_DIR}" "gha_zisk_setup") || return 1

        # If ZISK_SETUP_FILE is not set, define it using version from cargo-zisk
        ZISK_VERSION=$(echo "$(ensure cargo-zisk --version)" | awk '{print $2}')
        IFS='.' read -r major minor patch <<< "${ZISK_VERSION}"
        ZISK_SETUP_FILE="zisk-provingkey-pre-${major}.${minor}.0.tar.gz"
    else
        # We build the setup file name from the SETUP_VERSION variable
        ZISK_SETUP_FILE="zisk-provingkey-${SETUP_VERSION}.tar.gz"
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

    cd "$current_dir"

    success "Public proving key ${ZISK_SETUP_FILE} installed successfully!"
}

main