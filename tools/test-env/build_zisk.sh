#!/bin/bash

source ./utils.sh

# Sets crate_dir to the directory of the one package named $1 in the build's dependency graph
find_crate() {
    local paths
    paths="$(jq -r --arg n "$1" '.packages[] | select(.name == $n) | .manifest_path' <<< "$metadata")"
    if [[ -z "$paths" || "$paths" == *$'\n'* ]]; then
        err "Expected one $1 package in cargo metadata, found: ${paths:-none}"
        return 1
    fi
    crate_dir="$(dirname "$paths")"
}

copy_proofman_files() {
    local metadata crate_dir setup_dir vcs_info proofman_root js_dir circom other p

    metadata="$(cargo metadata --format-version 1)" || return 1

    # A crates.io crate holds only its own directory: take the rest of the repo from the
    # commit it was published from. A git or path dependency is a whole checkout.
    find_crate pil2-stark-setup || return 1
    setup_dir="$crate_dir"
    vcs_info="$setup_dir/.cargo_vcs_info.json"
    if [[ -f "$vcs_info" ]]; then
        if [[ "$(jq -r '.git.dirty // false' "$vcs_info")" == "true" ]]; then
            err "pil2-stark-setup was published from a dirty tree: no commit holds its files"
            return 1
        fi
        proofman_root="$(mktemp -d)"
        curl -fsSL --retry 3 \
            "https://codeload.github.com/0xPolygonHermez/pil2-proofman/tar.gz/$(jq -r .git.sha1 "$vcs_info")" \
            | tar -xz -C "$proofman_root" --strip-components=1 || return 1
    else
        proofman_root="$setup_dir/../.."
    fi

    # Remove files from a previous install, as ziskup does
    rm -rf "${ZISK_BIN_DIR}/setup" "${ZISK_BIN_DIR}/pil2-components" "${ZISK_BIN_DIR}/pil2-stark" \
        "${ZISK_DIR}/zisk/pilfflonk-js"

    for p in setup/circom setup/final_snark_circom setup/stark-recurser/stark2circom/circom_verifier \
             setup/stark-recurser/plonk2pil/pil pil2-components/lib/std/pil pil2-stark/src/goldilocks/src; do
        if [[ ! -d "$proofman_root/$p" ]]; then
            err "pil2-proofman has no $p (at $proofman_root)"
            return 1
        fi
        mkdir -p "${ZISK_BIN_DIR}/$(dirname "$p")"
        ensure cp -R "$proofman_root/$p" "${ZISK_BIN_DIR}/$p" || return 1
    done
    # proofman runs circom on Linux and circom_mac on macOS: keep this platform's.
    if [[ "${PLATFORM}" == "darwin" ]]; then
        circom=circom_mac other=circom
    else
        circom=circom other=circom_mac
    fi
    rm -f "${ZISK_BIN_DIR}/setup/circom/$other"
    if [[ ! -x "${ZISK_BIN_DIR}/setup/circom/$circom" ]]; then
        err "${ZISK_BIN_DIR}/setup/circom/$circom not found or not executable"
        return 1
    fi

    find_crate proofman-pilfflonk || return 1
    js_dir="$crate_dir/js"
    mkdir -p "${ZISK_DIR}/zisk/pilfflonk-js"
    ensure cp -R "$js_dir/bin" "$js_dir/src" "$js_dir/package.json" "${ZISK_DIR}/zisk/pilfflonk-js/" || return 1
    if [[ ! -f "${ZISK_DIR}/zisk/pilfflonk-js/bin/verify.js" ]]; then
        err "pilfflonk verify.js not found in ${ZISK_DIR}/zisk/pilfflonk-js"
        return 1
    fi
}

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

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR ZISK_BRANCH DISABLE_CLONE_REPO ONLY_CPU || return 1

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

    # Build the CUDA kernels for the current GPU's major compute capability,
    # unless this is a CPU-only build.
    CUDA_ENV=()
    if [[ "${ONLY_CPU}" != "1" ]]; then
        CUDA_ENV=(env CUDA_ARCHS="major")
    fi

    ensure "${CUDA_ENV[@]}" cargo build --release --target ${TARGET} ${BUILD_FEATURES}

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
    ensure cp target/${TARGET}/release/zisk-transpiler-riscv "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/zisk-coordinator "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/zisk-worker "${ZISK_BIN_DIR}" || return 1

    if [[ "${PLATFORM}" == "linux" ]]; then
        LIB_EXT="so"
    else
        LIB_EXT="dylib"
    fi

    ensure cp ziskup/ziskup "${ZISK_BIN_DIR}" || return 1
    ensure cp target/${TARGET}/release/libziskclib.a "${ZISK_BIN_DIR}" || return 1
    if [[ "${PLATFORM}" == "linux" ]]; then
        ensure cp target/zisk-libs/libziskc.a "${ZISK_BIN_DIR}" || return 1
    fi

    step "Copying emulator-asm files..."
    if [[ "${PLATFORM}" == "linux" ]]; then
        mkdir -p "${ZISK_DIR}/zisk/emulator-asm"
        ensure cp -r ./emulator-asm/src "${ZISK_DIR}/zisk/emulator-asm" || return 1
        ensure cp ./emulator-asm/Makefile "${ZISK_DIR}/zisk/emulator-asm" || return 1
        ensure cp -r ./lib-c "${ZISK_DIR}/zisk" || return 1
    fi

    step "Copying pil2-proofman files..."
    copy_proofman_files || return 1

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
