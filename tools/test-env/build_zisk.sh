#!/bin/bash

source ./utils.sh

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    total_steps=9

    if [[ "${PLATFORM}" == "linux" ]]; then
        TARGET="x86_64-unknown-linux-gnu"
    elif [[ "${PLATFORM}" == "darwin" ]]; then
        TARGET="aarch64-apple-darwin"
    else
        err "Unsupported platform: ${PLATFORM}"
        return 1
    fi

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    # If ZISK_GHA is set, force skip cloning pil2-proofman and use pil2-proofman dependency defined in zisk Cargo.toml
    if is_gha; then
        unset PIL2_PROOFMAN_BRANCH
    fi

    cd "${WORKSPACE_DIR}"

    step "Cloning pil2-proofman repository..."
    if [[ -n "$PIL2_PROOFMAN_BRANCH" ]]; then
        if [[ "$DISABLE_CLONE_REPO" == "1" ]]; then
            warn "Skipping cloning pil2-proofman repository as DISABLE_CLONE_REPO is set to 1"
        else
            # Remove existing directory if it exists
            rm -rf pil2-proofman
            # Clone pil2-proofman repository
            ensure git clone https://github.com/0xPolygonHermez/pil2-proofman.git || return 1
            cd pil2-proofman
            info "Checking out branch '$PIL2_PROOFMAN_BRANCH' for pil2-proofman..."
            ensure git checkout "$PIL2_PROOFMAN_BRANCH" || return 1
            cd ..
        fi
    else
        info "Skipping cloning pil2-proofman repository as PIL2_PROOFMAN_BRANCH is not defined"
    fi

    step "Setting up ZisK repository..."
    if [[ -n "${ZISK_REPO_DIR}" ]]; then
        info "Using ZisK repository defined in ZISK_REPO_DIR variable: ${ZISK_REPO_DIR}"
        ensure cd "${ZISK_REPO_DIR}"
    else
        if is_gha; then
            err "ZISK_GHA is set, but ZISK_REPO_DIR is not defined"
            return 1
        fi
        if [[ -n "$ZISK_BRANCH" ]]; then
            if [[ "$DISABLE_CLONE_REPO" == "1" ]]; then
                warn "Skipping cloning ZisK repository as DISABLE_CLONE_REPO is set to 1"
            else
                info "Cloning ZisK repository..."
                # Remove existing directory if it exists
                rm -rf zisk
                # Clone ZisK repository
                ensure git clone https://github.com/0xPolygonHermez/zisk.git || return 1
                ensure cd zisk
                # Check out the branch
                info "Checking out branch '$ZISK_BRANCH'..."
                ensure git checkout "$ZISK_BRANCH" || return 1
            fi
        else
            info "Skipping cloning ZisK repository as ZISK_BRANCH is not defined"
            ensure cd zisk
        fi
    fi

    if [[ -n "$PIL2_PROOFMAN_BRANCH" ]]; then
        info "Update ZisK cargo dependencies to use local pil2-proofman repo..."

        PIL2_PROOFMAN_DIR="${WORKSPACE_DIR}/pil2-proofman"

        replacements="
            proofman          | { path = \"${PIL2_PROOFMAN_DIR}/proofman\" }
            proofman-common   | { path = \"${PIL2_PROOFMAN_DIR}/common\" }
            proofman-macros   | { path = \"${PIL2_PROOFMAN_DIR}/macros\" }
            proofman-verifier | { path = \"${PIL2_PROOFMAN_DIR}/verifier\" }
            proofman-util     | { path = \"${PIL2_PROOFMAN_DIR}/util\" }
            pil-std-lib       | { path = \"${PIL2_PROOFMAN_DIR}/pil2-components/lib/std/rs\" }
            witness           | { path = \"${PIL2_PROOFMAN_DIR}/witness\" }
            fields            | { path = \"${PIL2_PROOFMAN_DIR}/fields\" }
        "

        if [[ "${PLATFORM}" == "linux" ]]; then
            # GNU sed
            SED_PARAMS=( -i -E )
        else
            # BSD sed (macOS)
            SED_PARAMS=( -i "" -E )
        fi

        # Iterate through the list of replacements and update Cargo.toml
        while IFS='|' read -r crate repl; do
            [[ -z "$crate" ]] && continue

            pattern="^${crate//[[:space:]]/} = \\{ git = \\\"https://github.com/0xPolygonHermez/pil2-proofman.git\\\", (tag|branch) = \\\".*\\\" *\\}"
            replacement="${crate//[[:space:]]/} = ${repl}"

            ensure sed "${SED_PARAMS[@]}" "s~${pattern}~${replacement}~" Cargo.toml
        done <<< "$replacements"
    fi

    step  "Building ZisK tools..."
    ensure cargo clean || return 1
    ensure cargo update || return 1
    BUILD_FEATURES=""
    if [[ "${BUILD_GPU}" == "1" ]]; then
        BUILD_FEATURES="--features gpu"
        warn "Building with GPU support..."
    fi
    if ! (cargo build --release --target ${TARGET} ${BUILD_FEATURES}); then
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
        ensure cargo build --release --target ${TARGET} ${BUILD_FEATURES} || return 1
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

    ensure cp target/${TARGET}/release/cargo-zisk "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/ziskemu "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/riscv2zisk "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/zisk-coordinator "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/zisk-worker "${ZISK_BIN_DIR}" || return 1

    if [[ "${PLATFORM}" == "linux" ]]; then
        LIB_EXT="so"
    else
        LIB_EXT="dylib"
    fi

    ensure cp target/${TARGET}/release/libzisk_witness.${LIB_EXT} "${ZISK_BIN_DIR}" || return 1
    ensure cp ziskup/ziskup "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/libziskclib.a "${ZISK_BIN_DIR}" || return 1

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
        info "Added $EXPORT_LINE to $PROFILE"
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

main
