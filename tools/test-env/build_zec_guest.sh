#!/bin/bash

source "./utils.sh"

# patch_zec_cargo_deps: repoint zisk-eth-client's git dependencies on ZisK at the
# local ZisK repo. Callers must run verify_cargo_lock first, since patching makes
# the lock unpinnable.
patch_zec_cargo_deps() {
    if [[ "${PLATFORM}" == "linux" ]]; then
        # GNU sed
        SED_PARAMS=( -i -E )
    else
        # BSD sed (macOS)
        SED_PARAMS=( -i "" -E )
    fi

    ZISK_REPO_DIR="$(get_zisk_repo_dir)"

    # (zisk-zkvm-interface was renamed from zkvm-interface; the on-disk dir is still zkvm-interface.)
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "zisk-sdk"            "${ZISK_REPO_DIR}/sdk"               || return 1
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "zisk-zkvm-interface" "${ZISK_REPO_DIR}/zkvm-interface"    || return 1
    patch_cargo_dep "${CLIENT_CARGO_TOML}" "ziskos"              "${ZISK_REPO_DIR}/ziskos/entrypoint" || return 1
}

build_reth_guest() {
    local guest_cargo_toml="${ZEC_GUEST_DIR}/Cargo.toml"

    step "Patching Cargo.toml files to use local zisk repo..."

    # `cargo-zisk build` takes no --locked, so the lock is pinned here instead, while
    # the checkout is still pristine. This ELF and the native ethproofs build both
    # deserialize the same pre-generated input files, so a third-party dependency
    # drifting between them silently desyncs those formats.
    verify_cargo_lock "${ZEC_GUEST_DIR}" || return 1

    patch_zec_cargo_deps || return 1
    patch_cargo_dep "${guest_cargo_toml}" "ziskos" "${ZISK_REPO_DIR}/ziskos/entrypoint" || return 1

    step "Building zec-reth ELF..."
    ensure cd "${ZEC_GUEST_DIR}" || return 1
    ensure cargo-zisk build --release || return 1
    cd "${WORKSPACE_DIR}" || return 1
}

# build_ziskethone_guest: regenerate the committed ziskethone ELF from the C++
# (evmone) sources in the third_party/ziskethone submodule.
#
# The ELF is a committed artifact, so the default is to use the one the clone
# already carries. A rebuild (REBUILD_ZISKETHONE_GUEST=1) needs cmake, and on a
# cold machine installs the xPack riscv-none-elf-gcc plus the patched GCC that
# provides -mzisk-dma (~10-15 min, then cached in $HOME).
build_ziskethone_guest() {
    if [[ "${REBUILD_ZISKETHONE_GUEST:-0}" != "1" ]]; then
        warn "Using the ELF committed in zisk-eth-client (set REBUILD_ZISKETHONE_GUEST=1 to rebuild it from the C++ sources)"
        return 0
    fi

    if ! command -v cmake >/dev/null 2>&1; then
        err "cmake not found: rebuilding the ziskethone guest needs it. Run install_deps.sh, or install cmake."
        return 1
    fi

    step "Patching Cargo.toml files to use local zisk repo..."

    # The build runs at the repo root (`cargo build -p guest-ziskethone`), so it is
    # the root lock that has to be pinned, and only while the checkout is pristine.
    verify_cargo_lock "${ZEC_REPO_DIR}" || return 1

    patch_zec_cargo_deps || return 1

    step "Building zec-ziskethone ELF..."
    ZISKETHONE_ELF_BEFORE="$(file_sha256 "${ZEC_ELF}" 2>/dev/null || echo "<missing>")"

    ensure cd "${ZEC_REPO_DIR}" || return 1

    # Run the cross-compile driver directly before handing over to cargo, which
    # hides build-script output: a cold toolchain install would otherwise sit
    # silent for ~15 minutes. It is idempotent, so the cargo build below re-checks
    # it in seconds and then copies the result into the committed ELF path.
    ensure bash ./crates/clients/ziskethone/guest/build-elf.sh || return 1

    cargo_locked_flags
    ensure cargo build -p guest-ziskethone --features ziskethone-rebuild-guest \
        ${CARGO_LOCKED_FLAGS[@]+"${CARGO_LOCKED_FLAGS[@]}"} || return 1

    cd "${WORKSPACE_DIR}" || return 1
}

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR ZISK_ETH_CLIENT_BRANCH DISABLE_CLONE_REPO ZEC_GUEST \
        REBUILD_ZISKETHONE_GUEST || return 1

    resolve_zec_guest || return 1

    ZEC_REPO_DIR="${WORKSPACE_DIR}/zisk-eth-client"
    CLIENT_CARGO_TOML="${ZEC_REPO_DIR}/Cargo.toml"

    current_step=1
    total_steps=2
    if [[ "${ZEC_GUEST}" != "ziskethone" || "${REBUILD_ZISKETHONE_GUEST:-0}" == "1" ]]; then
        total_steps=4
    fi

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

    # Also covers DISABLE_CLONE_REPO=1, where the checkout is reused as-is.
    ensure_submodules "zisk-eth-client" || return 1

    case "${ZEC_GUEST}" in
        reth)       build_reth_guest       || return 1 ;;
        ziskethone) build_ziskethone_guest || return 1 ;;
    esac

    step "Verifying ${ZEC_GUEST} ELF was generated..."
    if [[ ! -f "${ZEC_ELF}" ]]; then
        err "ELF file not found: ${ZEC_ELF}"
        return 1
    fi
    info "ELF file: ${ZEC_ELF}"

    # The committed ELF exists either way, so report whether the rebuild moved it.
    if [[ -n "${ZISKETHONE_ELF_BEFORE:-}" ]]; then
        local elf_after
        elf_after="$(file_sha256 "${ZEC_ELF}")"
        if [[ "${elf_after}" == "${ZISKETHONE_ELF_BEFORE}" ]]; then
            info "ELF is byte-identical to the one committed in zisk-eth-client (${elf_after})"
        else
            info "ELF changed by the rebuild: ${ZISKETHONE_ELF_BEFORE} -> ${elf_after}"
        fi
    fi

    cd "$current_dir"

    success "${ZEC_GUEST} ELF is ready!"
}

main
