#!/bin/bash

source ./utils.sh

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    total_steps=8

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

    # pil2-proofman is consumed as the git dependency pinned in the ZisK
    # Cargo.toml / Cargo.lock — it is never cloned or path-patched here.

    cd "${WORKSPACE_DIR}"

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
                ensure git clone --branch "$ZISK_BRANCH" --depth 1 --single-branch https://github.com/0xPolygonHermez/zisk.git || return 1
                ensure cd zisk
            fi
        else
            info "Skipping cloning ZisK repository as ZISK_BRANCH is not defined"
            ensure cd zisk
        fi
    fi

    step  "Building ZisK tools..."
    ensure cargo clean || return 1
    ensure cargo update || return 1

    # We build features in that way to be ready to support more feature in the future
    FEATURES=()
    if [[ "${ONLY_CPU}" == "1" ]]; then
        FEATURES+=("cpu-only")
        warn "Building with CPU only..."
    fi

    BUILD_FEATURES=""
    if (( ${#FEATURES[@]} > 0 )); then
        BUILD_FEATURES="--features $(IFS=,; echo "${FEATURES[*]}")"
    fi

    ensure cargo build --release --target ${TARGET} ${BUILD_FEATURES}

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
    ensure cp target/${TARGET}/release/cargo-zisk-dev "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/ziskemu "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/riscv2zisk "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/zisk-coordinator "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/zisk-worker "${ZISK_BIN_DIR}" || return 1

    if [[ "${PLATFORM}" == "linux" ]]; then
        LIB_EXT="so"
    else
        LIB_EXT="dylib"
    fi

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
    ensure cargo-zisk toolchain install || return 1

    step "Verifying toolchain installation..."
    rustup toolchain list | grep zisk || {
        err "ZisK toolchain not found."
        return 1
    }

    cd "$current_dir"

    success "ZisK build completed successfully!"
}

main
