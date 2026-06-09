#!/bin/bash
#
# Build the proving key for the test environment.
#
# This is a thin wrapper around tools/setup/build-setup.sh (the Rust /
# cargo-zisk pipeline). It no longer clones pil2-compiler / pil2-proofman /
# pil2-proofman-js or shells into node — compile-pil + setup run through
# cargo-zisk, and pil2-compiler is pulled via npm by build-setup.sh at the
# version pinned in pil2-proofman's package.json.
#
# Env vars (from .env / shell / Cargo.toml via load_env)
# ------------------------------------------------------
#   USE_CACHE_SETUP=1        Enable the local artifact cache. Passes
#                            --cache-dir "${OUTPUT_DIR}" to build-setup.sh, which
#                            keys the cache by <platform>/<input-hash>. A hit
#                            skips compile-pil + setup; a miss populates it.
#   DISABLE_RECURSIVE_SETUP=1  Build without aggregation (setup without -r).
#   ONLY_CPU=1               Skip the --gpu flag on check-setup.
#
# The proofman checkout build-setup.sh uses is whatever Cargo.toml resolves to
# (set up earlier by build_zisk.sh). Set PROOFMAN_DIR to override.

source ./utils.sh

# Directory holding this script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_step=1
    total_steps=4

    step "Loading environment variables..."
    load_env || return 1

    ZISK_REPO="$(get_zisk_repo_dir)"
    ensure cd "${ZISK_REPO}" || return 1

    step "Building setup (delegating to tools/test-env/setup_build.sh)..."
    build_flags=(--build-dir build)
    [[ "${DISABLE_RECURSIVE_SETUP}" == "1" ]] && build_flags+=(--no-aggregation)
    [[ "${USE_CACHE_SETUP}" == "1" ]] && build_flags+=(--cache-dir "${OUTPUT_DIR}")
    ensure "${SCRIPT_DIR}/setup_build.sh" "${build_flags[@]}" || return 1

    step "Copy provingKey directory to \$HOME/.zisk directory..."
    ensure mkdir -p "$HOME/.zisk" || return 1
    ensure rm -rf "$HOME/.zisk/provingKey" || return 1
    ensure cp -R "${ZISK_REPO}/build/provingKey" "$HOME/.zisk" || return 1

    step "Generate constant tree files..."
    local gpu_flag=""
    # Only enable GPU when not forced to CPU, not on macOS, and the installed
    # cargo-zisk is actually a GPU build (its `--version` description contains
    # "[gpu]", e.g. "cargo-zisk 0.18.0 [gpu] (790f9e2 ...)").
    if [[ "${ONLY_CPU:-}" != "1" ]] && [[ "${PLATFORM}" != "darwin" ]] && cargo-zisk --version 2>/dev/null | grep -q "\[gpu\]"; then
        gpu_flag="--gpu"
    fi
    local no_agg_flag=""
    [[ "${DISABLE_RECURSIVE_SETUP}" == "1" ]] && no_agg_flag="--no-aggregation"
    ensure cargo-zisk check-setup ${gpu_flag} ${no_agg_flag} || return 1

    success "ZisK setup completed successfully!"
}

main
