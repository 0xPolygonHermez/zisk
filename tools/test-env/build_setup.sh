#!/bin/bash
#
# Build (and optionally install) the ZisK setup (proving key).
#
# Env vars (loaded from .env / shell / Cargo.toml via load_env):
#   USE_CACHE_SETUP              Reuse/populate a local provingKey cache under
#                                $OUTPUT_DIR, keyed by <platform>/<input-hash>.
#   DISABLE_RECURSIVE_SETUP      Build without aggregation (setup without -r).
#   INSTALL_SETUP                Install the provingKey into $HOME/.zisk
#   INCLUDE_SNARK                After the proving key, also build the snark setup
#                                (provingKeySnark/). Needs state-machines/publics.json
#                                and the powers-of-tau file (PTAU_PATH).
#   DYLIB_INPUT_FILES            After the build, copy the inputs needed to compile
#                                the macOS dylib files into build/dylib_input.
#   RECURSIVE_JOBS / SETUP_JOBS  Setup pipeline concurrency.
#   HASH                         Hash function (default: Poseidon1).
#   PTAU_PATH                    Powers-of-tau file for the snark setup
#                                (default: ../powersOfTau28_hez_final_24.ptau);
#                                downloaded from PTAU_URL if missing.
#   PTAU_URL                     Download URL for the powers-of-tau file.
#   PROOFMAN_DIR                 Override the resolved pil2-proofman checkout.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/utils.sh"

HASH="${HASH:-Poseidon1}"

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
    current_step=1
    total_steps=2   # computing hash + building setup
    [[ "${INCLUDE_SNARK}" == "1" ]] && total_steps=$((total_steps + 1))
    [[ "${DYLIB_INPUT_FILES}" == "1" ]] && total_steps=$((total_steps + 1))
    [[ "${INSTALL_SETUP}" == "1" ]] && total_steps=$((total_steps + 1))

    info "Loading environment variables..."
    load_env || return 1

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
        step "Copying provingKey directory to \$HOME/.zisk directory..."
        ensure mkdir -p "$HOME/.zisk" || return 1
        ensure rm -rf "$HOME/.zisk/provingKey" || return 1
        ensure cp -R "${ZISK_REPO}/build/provingKey" "$HOME/.zisk" || return 1
    fi

    success "ZisK setup completed successfully!"

    # Emit the setup input hash
    echo "$local_hash"
}

main
