#!/bin/bash

source "./utils.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    total_steps=4

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR ZISK_ETHPROOFS_BRANCH DISABLE_CLONE_REPO ENABLE_HINTS ZEC_GUEST || return 1

    # Pins ENABLE_HINTS, which drives the --cfg zisk_hints build below.
    resolve_zec_guest || return 1

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

    ZISK_ETHPROOFS_DIR="zisk-ethproofs"
    ZISK_ETHPROOFS_BIN="${ZISK_ETHPROOFS_DIR}/target/release/ethproofs-client"
    ZISK_ETHPROOFS_CARGO_TOML="${ZISK_ETHPROOFS_DIR}/Cargo.toml"
    ZISK_REPO_DIR="$(get_zisk_repo_dir)"
    ZISK_ETH_CLIENT_REPO_DIR="${WORKSPACE_DIR}/zisk-eth-client"
    ZEC_CARGO_TOML="${ZISK_ETH_CLIENT_REPO_DIR}/Cargo.toml"

    step "Patching Cargo.toml files..."

    if [[ "${PLATFORM}" == "linux" ]]; then
        # GNU sed
        SED_PARAMS=( -i -E )
    else
        # BSD sed (macOS)
        SED_PARAMS=( -i "" -E )
    fi

    # `input` pulls input-ziskethone, whose rust-input-gen lives in the ziskethone
    # submodule; cargo cannot resolve the workspace without it.
    ensure_submodules "${ZISK_ETH_CLIENT_REPO_DIR}" || return 1

    # Checked while the checkout is still pristine: `input` is always repointed just
    # below, so from that point on the lock has to be re-resolved and --locked is off
    # the table.
    verify_cargo_lock "${ZISK_ETHPROOFS_DIR}" || return 1

    # Patch zisk-ethproofs Cargo.toml
    patch_cargo_dep "${ZISK_ETHPROOFS_CARGO_TOML}" "zisk-sdk"    "${ZISK_REPO_DIR}/sdk"               || return 1
    patch_cargo_dep "${ZISK_ETHPROOFS_CARGO_TOML}" "input"       "${ZISK_ETH_CLIENT_REPO_DIR}/crates/input"  || return 1

    # Patch zisk-eth-client Cargo.toml
    patch_cargo_dep "${ZEC_CARGO_TOML}" "zisk-sdk"            "${ZISK_REPO_DIR}/sdk"               || return 1
    patch_cargo_dep "${ZEC_CARGO_TOML}" "zisk-zkvm-interface" "${ZISK_REPO_DIR}/zkvm-interface"    || return 1
    patch_cargo_dep "${ZEC_CARGO_TOML}" "ziskos"              "${ZISK_REPO_DIR}/ziskos/entrypoint" || return 1

    step "Building ethproofs-client..."
    ensure cd "${ZISK_ETHPROOFS_DIR}" || return 1
    # Pin the committed Cargo.lock when the manifest was left untouched above, so a
    # stale lock fails loudly instead of being re-resolved: this build and the guest
    # ELF both deserialize the same pre-generated input files, and a third-party
    # dependency drifting between them desyncs those formats.
    cargo_locked_flags
    if [[ "${ENABLE_HINTS:-}" == "1" ]]; then
        ensure env RUSTFLAGS='--cfg zisk_hints --cfg zisk_hints_metrics --cfg zisk_hints_single_thread' cargo build --release ${CARGO_LOCKED_FLAGS[@]+"${CARGO_LOCKED_FLAGS[@]}"} || return 1
    else
        ensure cargo build --release ${CARGO_LOCKED_FLAGS[@]+"${CARGO_LOCKED_FLAGS[@]}"} || return 1
    fi
    cd "${WORKSPACE_DIR}" || return 1

    step "Verifying ethproofs-client binary was generated..."
    if [[ ! -f "${ZISK_ETHPROOFS_BIN}" ]]; then
        err "ethproofs-client binary not found: ${ZISK_ETHPROOFS_BIN}"
        return 1
    fi
    info "ethproofs-client binary generated: ${ZISK_ETHPROOFS_BIN}"

    cd "$current_dir"

    success "ethproofs-client has been successfully built!"
}

main
