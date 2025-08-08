#!/bin/bash

source ./utils.sh

main() {
    current_step=1
    total_steps=10

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 1

    source $HOME/.cargo/env

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
    else
        info "Skipping cloning pil2-proofman repository. Pulling existing repository"
        ensure cd pil2-proofman
        ensure git pull
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
    else
        info "Skipping cloning zisk repository. Pulling existing repository"
        ensure cd zisk
        ensure git pull || return 1
    fi

    if [[ -n "$PIL2_PROOFMAN_BRANCH" ]]; then
        step "Update ZisK cargo dependencies to use local pil2-proofman repo..."
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

    step "Copying binaries to ${HOME}/.zisk/bin..."
    mkdir -p "$HOME/.zisk/bin"
    ensure cp target/release/cargo-zisk target/release/ziskemu target/release/riscv2zisk \
        target/release/libzisk_witness.so target/release/libziskclib.a "$HOME/.zisk/bin" || return 1

    if [[ -f "precompiles/sha256f/src/sha256f_script.json" ]]; then
        err "sha256f_script.json file found. This should exist only if building version 0.9.0"
        return 1
    fi

    if [[ -f "precompiles/keccakf/src/keccakf_script.json" ]]; then
        err "keccakf_script.json file found. This should exist only if building version 0.7.0 or earlier"
        return 1
    fi

    step "Copying emulator-asm files..."
    mkdir -p "$HOME/.zisk/zisk/emulator-asm"
    ensure cp -r ./emulator-asm/src "$HOME/.zisk/zisk/emulator-asm" || return 1
    ensure cp ./emulator-asm/Makefile "$HOME/.zisk/zisk/emulator-asm" || return 1
    ensure cp -r ./lib-c $HOME/.zisk/zisk || return 1
    step "Adding ~/.zisk/bin to PATH..."

    # Add export line to .bashrc if it doesn't exist
    EXPORT_PATH='export PATH="$PATH:$HOME/.zisk/bin"'
    grep -Fxq "$EXPORT_PATH" "$HOME/.bashrc" || echo "$EXPORT_PATH" >> "$HOME/.bashrc"

    # Ensure the PATH is updated in the current session
    eval "$EXPORT_PATH"

    step "Installing ZisK Rust toolchain..."
    ensure cargo-zisk sdk install-toolchain || return 1

    step "Verifying toolchain installation..."
    rustup toolchain list | grep zisk || {
        err "ZisK toolchain not found."
        return 1
    }

    cd ..
}

main || return 1
