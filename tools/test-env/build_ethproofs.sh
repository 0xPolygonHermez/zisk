#!/bin/bash

source "./utils.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    total_steps=5

    step "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR ZISK_ETHPROOFS_BRANCH DISABLE_CLONE_REPO || return 1

    cd "${WORKSPACE_DIR}" || return 1

    step "Cloning zisk-ethproofs repository..."
    if [[ "$DISABLE_CLONE_REPO" == "1" ]]; then
        warn "Skipping cloning zisk-ethproofs repository as DISABLE_CLONE_REPO is set to 1"
    else
        # Remove existing directory if it exists
        rm -rf zisk-ethproofs
        # Clone zisk-ethproofs repository
        if [[ -n "$ZISK_ETHPROOFS_BRANCH" ]]; then
            info "Cloning branch '$ZISK_ETHPROOFS_BRANCH' of zisk-ethproofs..."
            ensure git clone --branch "$ZISK_ETHPROOFS_BRANCH" --single-branch --depth 1 https://github.com/0xPolygonHermez/zisk-ethproofs.git || return 1
        else
            ensure git clone --depth 1 --single-branch https://github.com/0xPolygonHermez/zisk-ethproofs.git || return 1
        fi
    fi

    CLIENT_DIR="zisk-ethproofs"
    CLIENT_BIN="${CLIENT_DIR}/target/release/ethproofs-client"
    CLIENT_CARGO_TOML="${CLIENT_DIR}/Cargo.toml"

    step "Patching Cargo.toml files to use local zisk repo..."

    if [[ "${PLATFORM}" == "linux" ]]; then
        # GNU sed
        SED_PARAMS=( -i -E )
    else
        # BSD sed (macOS)
        SED_PARAMS=( -i "" -E )
    fi

    # Resolve the absolute path to the ZisK repo (handles ZISK_REPO_DIR overrides used by GHA),
    # then repoint each git dependency to its local crate so the build uses this repo.
    ZISK_REPO_DIR="$(get_zisk_repo_dir)"
    ZISK_ETH_CLIENT_REPO_DIR="${WORKSPACE_DIR}/zisk-eth-client"

    # Client Cargo.toml: depends on zisk-common, ziskos and zisk-sdk.
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "zisk-common" "${ZISK_REPO_DIR}/common"            || return 1
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "ziskos"      "${ZISK_REPO_DIR}/ziskos/entrypoint" || return 1
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "zisk-sdk"    "${ZISK_REPO_DIR}/sdk"               || return 1

    patch_cargo_dep "${CLIENT_CARGO_TOML}" "input"       "${ZISK_ETH_CLIENT_REPO_DIR}/crates/input"  || return 1

    step "Building ethproofs-client..."
    ensure cd "${CLIENT_DIR}" || return 1
    ensure env RUSTFLAGS='--cfg zisk_hints --cfg zisk_hints_metrics --cfg zisk_hints_single_thread' cargo build --release || return 1
    cd "${WORKSPACE_DIR}" || return 1

    step "Verifying ethproofs-client binary was generated..."
    if [[ ! -f "${CLIENT_BIN}" ]]; then
        err "ethproofs-client binary not found: ${CLIENT_BIN}"
        return 1
    fi
    info "ethproofs-client binary generated: ${CLIENT_BIN}"

    cd "$current_dir"

    success "ethproofs-client has been successfully built!"
}

main
