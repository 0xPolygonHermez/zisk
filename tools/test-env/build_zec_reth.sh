#!/bin/bash

source "./utils.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    total_steps=5

    step "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR ZISK_ETH_CLIENT_BRANCH DISABLE_CLONE_REPO || return 1

    cd "${WORKSPACE_DIR}" || return 1

    step "Cloning zisk-eth-client repository..."
    if [[ "$DISABLE_CLONE_REPO" == "1" ]]; then
        warn "Skipping cloning zisk-eth-client repository as DISABLE_CLONE_REPO is set to 1"
    else
        # Remove existing directory if it exists
        rm -rf zisk-eth-client
        # Clone zisk-eth-client repository
        if [[ -n "$ZISK_ETH_CLIENT_BRANCH" ]]; then
            info "Cloning branch '$ZISK_ETH_CLIENT_BRANCH' of zisk-eth-client..."
            ensure git clone --branch "$ZISK_ETH_CLIENT_BRANCH" --single-branch --depth 1 https://github.com/0xPolygonHermez/zisk-eth-client.git || return 1
        else
            ensure git clone --depth 1 --single-branch https://github.com/0xPolygonHermez/zisk-eth-client.git || return 1
        fi
    fi

    GUEST_DIR="zisk-eth-client/bin/guests/stateless-validator-reth"
    ELF_FILE="${GUEST_DIR}/target/elf/riscv64ima-zisk-zkvm-elf/release/zec-reth"
    GUEST_CARGO_TOML="${GUEST_DIR}/Cargo.toml"
    CLIENT_CARGO_TOML="zisk-eth-client/Cargo.toml"

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

    # Guest Cargo.toml: only depends on ziskos.
    patch_cargo_dep "${GUEST_CARGO_TOML}" "ziskos" "${ZISK_REPO_DIR}/ziskos/entrypoint" || return 1

    # Client Cargo.toml: depends on zisk-sdk, zkvm-interface and ziskos.
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "zisk-sdk"       "${ZISK_REPO_DIR}/sdk"               || return 1
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "zkvm-interface" "${ZISK_REPO_DIR}/zkvm-interface"    || return 1
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "ziskos"         "${ZISK_REPO_DIR}/ziskos/entrypoint" || return 1

    step "Building zec-reth ELF..."
    ensure cd "${GUEST_DIR}" || return 1
    ensure git submodule update --init --recursive || return 1
    ensure cargo-zisk build --release || return 1
    cd "${WORKSPACE_DIR}" || return 1

    step "Verifying zec-reth ELF was generated..."
    if [[ ! -f "${ELF_FILE}" ]]; then
        err "ELF file not found: ${ELF_FILE}"
        return 1
    fi
    info "ELF file generated: ${ELF_FILE}"

    cd "$current_dir"

    success "zec-reth ELF has been successfully built!"
}

main
