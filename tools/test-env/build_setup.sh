#!/bin/bash
#
# Build (and optionally install) the ZisK setup (proving key).
#
# Env vars (loaded from .env / shell / Cargo.toml via load_env):
#   USE_CACHE_SETUP              Reuse/populate a local provingKey cache under
#                                $OUTPUT_DIR, keyed by <platform>/<input-hash>.
#   FORCE_SETUP_BUILD            Ignore a cache hit and rebuild the setup (the
#                                fresh build then refreshes the cache entry).
#   DISABLE_RECURSIVE_SETUP      Build without aggregation (setup without -r).
#   INSTALL_SETUP                Install the provingKey into $HOME/.zisk, plus the
#                                provingKeySnark when the build produced one.
#   INCLUDE_SNARK                After the proving key, also build the snark setup
#                                (provingKeySnark/). Needs state-machines/publics.json
#                                and the powers-of-tau file (PTAU_PATH).
#   DYLIB_INPUT_FILES            After the build, copy the inputs needed to compile
#                                the macOS dylib files into build/dylib_input.
#   RECURSIVE_JOBS / SETUP_JOBS  Setup pipeline concurrency.
#   HASH                         Hash function (default: Poseidon1).
#   PTAU_PATH                    Powers-of-tau file for the snark setup
#                                (default: ../powersOfTau28_hez_final_24.ptau).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/utils.sh"

# Copy the inputs needed to compile the macOS dylib files into
# build_dir/dylib_input, preserving the provingKey/ provingKeySnark/ layout.
copy_dylib_input() {
    local build_dir="$1"
    local dest
    dest="$(cd "$build_dir" && pwd)/dylib_input"
    ensure rm -rf "$dest" || return 1

    ( cd "$build_dir" || exit 1
      for tree in provingKey provingKeySnark; do
          [ -d "$tree" ] || continue
          find "$tree" \( -name '*.cpp' -o -name '*.so' -o -name '*.globalInfo.json' -o -name '*.dat' \) -print0 \
            | while IFS= read -r -d '' f; do
                mkdir -p "$dest/$(dirname "$f")"
                cp "$f" "$dest/$f"
              done
      done
    ) || return 1
}

main() {
    info "▶️  Running $(basename "$0") script..."

    local build_dir="build"
    local local_hash=""

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR PIL2_COMPILER_BRANCH USE_CACHE_SETUP FORCE_SETUP_BUILD \
        DISABLE_RECURSIVE_SETUP INSTALL_SETUP INCLUDE_SNARK DYLIB_INPUT_FILES \
        HASH PTAU_PATH RECURSIVE_JOBS SETUP_JOBS || return 1

    # Default the hash function when neither the shell, .env, nor Cargo.toml set
    # it. Exported so the setup_build.sh child process inherits it.
    export HASH="${HASH:-Poseidon1}"

    current_step=1
    total_steps=2   # computing hash + building setup
    [[ "${INCLUDE_SNARK}" == "1" ]] && total_steps=$((total_steps + 1))
    [[ "${DYLIB_INPUT_FILES}" == "1" ]] && total_steps=$((total_steps + 1))
    [[ "${INSTALL_SETUP}" == "1" ]] && total_steps=$((total_steps + 1))


    ZISK_REPO="$(get_zisk_repo_dir)"
    # Export so child tooling resolves the repo root from this, not its own location.
    export ZISK_REPO_DIR="${ZISK_REPO}"
    ensure cd "${ZISK_REPO}" || return 1

    build_flags=(--build-dir build --gen-exps --exps-arch major)
    [[ "${DISABLE_RECURSIVE_SETUP}" == "1" ]] && build_flags+=(--no-aggregation)
    [[ "${USE_CACHE_SETUP}" == "1" ]] && build_flags+=(--cache-dir "${OUTPUT_DIR}")

    [[ -n "${PIL2_COMPILER_BRANCH}" ]] && build_flags+=(--pil2-compiler-branch "${PIL2_COMPILER_BRANCH}")
    # setup_build.sh (build / no-aggregation) emits the input hash as its final
    # stdout line. tee streams the build output to the terminal while we keep a
    # copy to read that last line from; PIPESTATUS[0] carries setup_build.sh's
    # real exit status (no pipefail here, so tail's status would otherwise mask it).
    local setup_log; setup_log="$(mktemp)"
    "${SCRIPT_DIR}/setup_build.sh" "${build_flags[@]}" | tee "$setup_log"
    if [[ "${PIPESTATUS[0]}" -ne 0 ]]; then
        rm -f "$setup_log"
        err "setup_build.sh failed"
        return 1
    fi
    local_hash="$(tail -n1 "$setup_log")"
    rm -f "$setup_log"

    if [[ "${INCLUDE_SNARK}" == "1" ]]; then
        step "Building snark setup..."
        build_flags=(--build-dir build --snark)
        ensure "${SCRIPT_DIR}/setup_build.sh" "${build_flags[@]}" || return 1
    fi

    if [[ "${DYLIB_INPUT_FILES}" == "1" ]]; then
        step "Copying dylib input files to $build_dir/dylib_input..."
        copy_dylib_input "$build_dir" || return 1
    fi

    if [[ "${INSTALL_SETUP}" == "1" ]]; then
        step "Copying proving key directories to \$HOME/.zisk directory..."
        ensure mkdir -p "$HOME/.zisk" || return 1
        ensure rm -rf "$HOME/.zisk/provingKey" || return 1
        ensure cp -R "${ZISK_REPO}/build/provingKey" "$HOME/.zisk" || return 1

        # The snark proving key goes with it whenever the build produced one
        # (INCLUDE_SNARK=1), to the same place `ziskup setup_snark` installs it.
        if [[ -d "${ZISK_REPO}/build/provingKeySnark" ]]; then
            ensure rm -rf "$HOME/.zisk/provingKeySnark" || return 1
            ensure cp -R "${ZISK_REPO}/build/provingKeySnark" "$HOME/.zisk" || return 1
        fi
    fi

    success "ZisK setup completed successfully!"

    # Emit the setup input hash
    echo "$local_hash"
}

main
