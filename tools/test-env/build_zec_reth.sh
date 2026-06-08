#!/bin/bash

source "./utils.sh"

# patch_cargo_dep: Repoint a git dependency in a Cargo.toml to a local path.
# Comments out the existing `<crate> = { git = ... }` line and inserts (idempotently)
# a `<crate> = { path = "<local_path>" }` entry right after it.
# Relies on the SED_PARAMS global set up in main().
# Usage: patch_cargo_dep <cargo_toml> <crate_name> <local_path>
patch_cargo_dep() {
    local cargo_toml="$1"
    local crate="$2"
    local dep_path="$3"

    if [[ ! -f "${cargo_toml}" ]]; then
        err "Cargo.toml not found: ${cargo_toml}"
        return 1
    fi
    if [[ ! -f "${dep_path}/Cargo.toml" ]]; then
        err "Local path for '${crate}' not found: ${dep_path}/Cargo.toml. Make sure the ZisK repo is available."
        return 1
    fi

    # Escape regex-special characters in the crate name for sed/grep patterns.
    local crate_re
    crate_re=$(printf '%s' "${crate}" | sed 's/[.[\*^$+?{}|()\/]/\\&/g')

    local new_line="${crate} = { path = \"${dep_path}\" }"

    # Comment line:   <crate> = { git = "https://github.com/0xPolygonHermez/zisk.git", branch = "..." }
    ensure sed "${SED_PARAMS[@]}" \
        "s~^${crate_re}[[:space:]]*=[[:space:]]*[{][[:space:]]*git~# &~" \
        "${cargo_toml}" || return 1

    # Remove any previously-added uncommented path entry (idempotent reruns).
    ensure sed "${SED_PARAMS[@]}" \
        "/^${crate_re}[[:space:]]*=[[:space:]]*[{][[:space:]]*path/d" \
        "${cargo_toml}" || return 1

    # Insert a new uncommented path entry right after the (now) commented git dependency,
    # pointing to the resolved absolute path so it works regardless of where the ZisK repo lives.
    ensure sed "${SED_PARAMS[@]}" \
        "/^#[[:space:]]*${crate_re}[[:space:]]*=[[:space:]]*[{][[:space:]]*git/a\\
${new_line}" \
        "${cargo_toml}" || return 1

    # Verify the patch was applied correctly.
    if ! grep -qE "^#[[:space:]]*${crate_re}[[:space:]]*=[[:space:]]*[{][[:space:]]*git" "${cargo_toml}"; then
        err "Failed to comment '${crate} = { git = ... }' line in ${cargo_toml}"
        return 1
    fi
    if ! grep -qF "${new_line}" "${cargo_toml}"; then
        err "Failed to add ${crate} path entry pointing to ${dep_path} in ${cargo_toml}"
        return 1
    fi
}

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
