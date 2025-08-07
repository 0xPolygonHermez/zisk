#!/bin/bash

source ./utils.sh

main() {
    current_dir=$(pwd)

    current_step=1
    total_steps=9

    source $HOME/.cargo/env

    ZISK_DIR="$HOME/.zisk"
    ZISK_BIN_DIR="$ZISK_DIR/bin"

    get_platform || return 1
    get_shell_and_profile || return 1

    # If ZISK_GHA is set to 1, then ZISK_BRANCH must be defined
    if [[ "$ZISK_GHA" == "1" ]]; then
        if [[ -z "$ZISK_BRANCH" ]]; then
            err "ZISK_GHA is set to 1, but ZISK_BRANCH is not defined. Aborting"
            return 1
        fi
        info "Executing build_zisk.sh script"
        # If ZISK_GHA is set, skip loading .env file as env variables are already set from docker command line
        step "Skipping loading .env file since ZISK_GHA is set to 1"
    else
        step "Loading environment variables..."
        # Load environment variables from .env file
        load_env || return 1
        confirm_continue || return 1
    fi

    mkdir -p "${HOME}/work"
    cd "${HOME}/work"

    step "Cloning pil2-proofman repository..."
    if [[ -n "$PIL2_PROOFMAN_BRANCH" ]]; then
        # Remove existing directory if it exists
        rm -rf pil2-proofman
        # Clone pil2-proofman repository
        ensure git clone https://github.com/0xPolygonHermez/pil2-proofman.git || return 1
        cd pil2-proofman
        info "Checking out branch '$PIL2_PROOFMAN_BRANCH' for pil2-proofman..."
        ensure git checkout "$PIL2_PROOFMAN_BRANCH" || return 1
        cd ..
    fi

    step  "Cloning ZisK repository..."
    if [[ -n "$ZISK_BRANCH" ]]; then
        # Remove existing directory if it exists
        rm -rf zisk
        # Clone ZisK repository
        ensure git clone https://github.com/0xPolygonHermez/zisk.git || return 1
        ensure cd zisk
        # Check out the branch
        info "Checking out branch '$ZISK_BRANCH'..."
        ensure git checkout "$ZISK_BRANCH" || return 1
    fi

    if [[ -n "$PIL2_PROOFMAN_BRANCH" ]]; then
        info "Update ZisK cargo dependencies to use local pil2-proofman repo..."
        # Dependencies to be replaced
        declare -A replacements=(
        ["proofman"]='{ path = "../pil2-proofman/proofman" }'
        ["proofman-common"]='{ path = "../pil2-proofman/common" }'
        ["proofman-macros"]='{ path = "../pil2-proofman/macros" }'
        ["proofman-util"]='{ path = "../pil2-proofman/util" }'
        ["pil-std-lib"]='{ path = "../pil2-proofman/pil2-components/lib/std/rs" }'
        ["witness"]='{ path = "../pil2-proofman/witness" }'
        ["fields"]='{ path = "../pil2-proofman/fields" }'
        )
        # Iterate over the replacements and update the Cargo.toml file
        for crate in "${!replacements[@]}"; do
            pattern="^$crate = \\{ git = \\\"https://github.com/0xPolygonHermez/pil2-proofman.git\\\", (tag|branch) = \\\".*\\\" *\\}"
            replacement="$crate = ${replacements[$crate]}"
            sed -i -E "s~$pattern~$replacement~" Cargo.toml
        done
    fi

    step  "Building ZisK tools..."
    ensure cargo clean || return 1
    ensure cargo update || return 1
    BUILD_FEATURES=""
    if [[ "${BUILD_GPU}" == "1" ]]; then
        BUILD_FEATURES="--features gpu"
        warn "Building with GPU support..."
    fi
    if ! (cargo build --release ${BUILD_FEATURES}); then
        warn "Build failed. Trying to fix missing stddef.h..."

        stddef_path=$(find /usr -name "stddef.h" 2>/dev/null | head -n 1)
        if [ -z "$stddef_path" ]; then
            err "stddef.h not found. You may need to install gcc headers."
            return 1
        fi

        include_dir=$(dirname "$stddef_path")
        export C_INCLUDE_PATH=$include_dir
        export CPLUS_INCLUDE_PATH=$C_INCLUDE_PATH

        info  "Retrying build..."
        ensure cargo build --release ${BUILD_FEATURES} || return 1
    fi

    step "Copying binaries to ${ZISK_BIN_DIR}..."
    mkdir -p "${ZISK_BIN_DIR}"

    if [[ -f "precompiles/sha256f/src/sha256f_script.json" ]]; then
        err "sha256f_script.json file found. This should exist only if building version 0.9.0"
        return 1
    fi

    if [[ -f "precompiles/keccakf/src/keccakf_script.json" ]]; then
        err "keccakf_script.json file found. This should exist only if building version 0.7.0 or earlier"
        return 1
    fi

    ensure cp target/release/cargo-zisk "${ZISK_BIN_DIR}" || return 1
    ensure cp target/release/ziskemu    "${ZISK_BIN_DIR}" || return 1
    ensure cp target/release/riscv2zisk "${ZISK_BIN_DIR}" || return 1

    if [[ "${PLATFORM}" == "linux" ]]; then
        ensure cp target/release/libzisk_witness.so "${ZISK_BIN_DIR}" || return 1
        ensure cp ziskup/ziskup                     "${ZISK_BIN_DIR}" || return 1
        ensure cp target/release/libziskclib.a      "${ZISK_BIN_DIR}" || return 1
    fi

    step "Copying emulator-asm files..."
    if [[ "${PLATFORM}" == "linux" ]]; then
        mkdir -p "${ZISK_DIR}/zisk/emulator-asm"
        ensure cp -r ./emulator-asm/src "${ZISK_DIR}/zisk/emulator-asm" || return 1
        ensure cp ./emulator-asm/Makefile "${ZISK_DIR}/zisk/emulator-asm" || return 1
        ensure cp -r ./lib-c "${ZISK_DIR}/zisk" || return 1
    fi

    step "Adding ${ZISK_BIN_DIR} to PATH..."
    EXPORT_PATH="export PATH=\"$PATH:$ZISK_BIN_DIR\""
    # Ensure the PATH is updated in the current session
    eval "$EXPORT_PATH"

    EXPORT_LINE="export PATH=\"\$PATH:$ZISK_BIN_DIR\""
    # Ensure the PATH is updated in the shell profile
    if ! grep -Fxq "$EXPORT_LINE" "$PROFILE"; then
        echo "$EXPORT_LINE" >> "$PROFILE"
    fi

    step "Installing ZisK Rust toolchain..."
    ensure cargo-zisk sdk install-toolchain || return 1

    step "Verifying toolchain installation..."
    rustup toolchain list | grep zisk || {
        err "ZisK toolchain not found."
        return 1
    }

    cd "$current_dir"

    success "ZisK build completed successfully!"
}

main || return 1
