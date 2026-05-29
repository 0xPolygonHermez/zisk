#!/bin/bash

source "./utils.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    total_steps=5

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

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

    step "Patching ${GUEST_CARGO_TOML} to use local zisk repo..."
    if [[ ! -f "${GUEST_CARGO_TOML}" ]]; then
        err "Cargo.toml not found: ${GUEST_CARGO_TOML}"
        return 1
    fi

    if [[ "${PLATFORM}" == "linux" ]]; then
        # GNU sed
        SED_PARAMS=( -i -E )
    else
        # BSD sed (macOS)
        SED_PARAMS=( -i "" -E )
    fi

    # Resolve the absolute path to ziskos/entrypoint (handles ZISK_REPO_DIR overrides used by GHA)
    ZISKOS_ENTRYPOINT_PATH="$(get_zisk_repo_dir)/ziskos/entrypoint"
    if [[ ! -f "${ZISKOS_ENTRYPOINT_PATH}/Cargo.toml" ]]; then
        err "ZisK entrypoint Cargo.toml not found: ${ZISKOS_ENTRYPOINT_PATH}/Cargo.toml. Make sure the ZisK repo is available."
        return 1
    fi

    # Comment line:   ziskos = { git = "https://github.com/0xPolygonHermez/zisk.git", branch = "..." }
    ensure sed "${SED_PARAMS[@]}" \
        's~^ziskos[[:space:]]*=[[:space:]]*[{][[:space:]]*git~# &~' \
        "${GUEST_CARGO_TOML}" || return 1

    # Remove any previously-added uncommented ziskos path entry (idempotent reruns)
    ensure sed "${SED_PARAMS[@]}" \
        '/^ziskos[[:space:]]*=[[:space:]]*[{][[:space:]]*path/d' \
        "${GUEST_CARGO_TOML}" || return 1

    # Append a new uncommented ziskos path entry below the existing commented one,
    # pointing to the resolved absolute path so it works regardless of where the ZisK
    # repo lives (e.g. in GHA the container mounts it outside WORKSPACE_DIR)
    ZISKOS_NEW_LINE="ziskos = { path = \"${ZISKOS_ENTRYPOINT_PATH}\" }"
    ensure sed "${SED_PARAMS[@]}" \
        "/^#[[:space:]]*ziskos[[:space:]]*=[[:space:]]*[{][[:space:]]*path/a\\
${ZISKOS_NEW_LINE}" \
        "${GUEST_CARGO_TOML}" || return 1

    # Verify the patch was applied correctly
    if ! grep -qE '^#[[:space:]]*ziskos[[:space:]]*=[[:space:]]*[{][[:space:]]*git' "${GUEST_CARGO_TOML}"; then
        err "Failed to comment 'ziskos = { git = ... }' line in ${GUEST_CARGO_TOML}"
        return 1
    fi
    if ! grep -qE '^#[[:space:]]*ziskos[[:space:]]*=[[:space:]]*[{][[:space:]]*path' "${GUEST_CARGO_TOML}"; then
        err "Original commented 'ziskos = { path = ... }' line missing in ${GUEST_CARGO_TOML}"
        return 1
    fi
    if ! grep -qF "${ZISKOS_NEW_LINE}" "${GUEST_CARGO_TOML}"; then
        err "Failed to add ziskos path entry pointing to ${ZISKOS_ENTRYPOINT_PATH} in ${GUEST_CARGO_TOML}"
        return 1
    fi

    step "Building zec-reth ELF..."
    ensure cd "${GUEST_DIR}" || return 1
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
