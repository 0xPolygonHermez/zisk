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

# Compile pil/zisk.pil into pil/zisk.pilout
run_compile_pil() {
    ensure cargo run --release --bin cargo-zisk-dev -- proofman-setup compile-pil \
        --pil pil/zisk.pil \
        --include "$INCLUDE_PATHS" \
        --output pil/zisk.pilout \
        --fixed-dir tmp/fixed \
        --fixed-to-file \
        --no-proto-fixed-data || return 1
}

# Regenerate pil/src/pil_helpers/ from the compiled pilout
run_pil_helpers() {
    ensure cargo run --release --manifest-path "$PROOFMAN_DIR/Cargo.toml" --bin proofman-cli -- \
        pil-helpers \
            --pilout pil/zisk.pilout \
            --path pil/src \
            -o || return 1
}

# Build the snark setup (provingKeySnark/) on top of an existing build/provingKey.
run_setup_snark() {
    local build_dir="$1"

    [ -d "$build_dir/provingKey" ] || { err "$build_dir/provingKey not found — build the proving key first"; return 1; }

    local publics_info="state-machines/publics.json"
    [ -f "$publics_info" ] || { err "Missing $publics_info — final.circom needs the publics layout"; return 1; }

    # Powers-of-tau: download (~18 GB) to PTAU_PATH from PTAU_URL if not present.
    local ptau_path="${PTAU_PATH:-../powersOfTau28_hez_final_24.ptau}"
    local ptau_url="${PTAU_URL:-https://storage.googleapis.com/zkevm/ptau/powersOfTau28_hez_final_24.ptau}"
    if [ ! -f "$ptau_path" ]; then
        info "Downloading powers-of-tau (~18 GB) to $ptau_path..."
        ensure curl -fL -o "$ptau_path" "$ptau_url" || return 1
    fi

    ensure cargo run --release --bin cargo-zisk-dev -- proofman-setup setup-snark \
        --build-dir "$build_dir" \
        --publics-info "$publics_info" \
        --powers-of-tau "$ptau_path" || return 1
}

# Generate the per-AIR Q-expression CUDA kernels (.exps.so) into <build-dir>/provingKey/.
run_gen_exps() {
  if ! command -v nvcc >/dev/null 2>&1; then
    err "gen-exps skipped, nvcc not found" >&2; return 1;
  fi
  info "Proofman setup gen-exps (arch: major)"
  cargo run --release --bin cargo-zisk-dev -- proofman-setup gen-exps \
    --proving-key "$BUILD_DIR/provingKey" \
    --arch major
}

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

# Copy the freshly built provingKey into the local artifact cache (only when
# USE_CACHE_SETUP=1) so later runs with the same setup hash can reuse it.
cache_proving_key() {
    local build_dir="$1"
    local cache_entry="$2"

    [[ "${USE_CACHE_SETUP}" == "1" ]] || return 0

    info "Caching provingKey to $cache_entry"
    ensure rm -rf "$cache_entry" || return 1
    ensure mkdir -p "$cache_entry" || return 1
    ensure cp -R "$build_dir/provingKey" "$cache_entry/provingKey" || return 1
}

main() {
    info "▶️  Running $(basename "$0") script..."

    local build_dir="build"
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

    PROOFMAN_DIR="${PROOFMAN_DIR:-$(resolve_proofman_dir)}" || return 1
    export PROOFMAN_DIR   # honored if preset; also lets setup_hash.sh reuse it
    info "Proofman dir: $PROOFMAN_DIR"

    VERSION="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$ZISK_REPO/Cargo.toml")"
    INCLUDE_PATHS="pil,${PROOFMAN_DIR}/pil2-components/lib/std/pil,state-machines,precompiles"

    local recursive_flag=(--recursive)
    local mode_label="setup (recursive)"
    if [[ "${DISABLE_RECURSIVE_SETUP}" == "1" ]]; then
        recursive_flag=()
        mode_label="setup (no aggregation)"
    fi
    info "Version: $VERSION  mode: $mode_label"

    # setup_hash.sh generates the frops fixed data and returns the setup hash.
    step "Computing setup hash..."
    local local_hash
    local_hash="$("$SCRIPT_DIR/setup_hash.sh")" || return 1
    info "Local setup hash: $local_hash"

    # Local artifact cache lookup
    local cache_hit=0 gen_exps_on_hit=0 cache_entry=""
    if [[ "${USE_CACHE_SETUP}" == "1" ]]; then
        local short_hash cache_key
        # Mode is part of the key: the input hash doesn't encode -r, so a recursive
        # and a no-aggregation build must not collide.
        short_hash="${local_hash:0:4}${local_hash: -4}"
        cache_key="$short_hash"
        [[ "${DISABLE_RECURSIVE_SETUP}" == "1" ]] && cache_key="${short_hash}-no-aggregation"
        cache_entry="${OUTPUT_DIR}/${PLATFORM}/${cache_key}"

        if [[ -d "$cache_entry/provingKey" ]]; then
            ensure rm -rf "$build_dir/provingKey" || return 1
            ensure mkdir -p "$build_dir" || return 1
            ensure cp -R "$cache_entry/provingKey" "$build_dir/provingKey" || return 1
            cache_hit=1
        fi

        # gen-exps kernels are baked into the provingKey during the miss build (see
        # below) and cached alongside it. With --exps-arch major they carry SASS +
        # forward-PTX for every major arch, so a cached copy is host-independent and
        # reusable as-is. On a cache hit we therefore reuse the cached .exps.so
        # instead of regenerating them — UNLESS the cache predates kernel-caching
        # and has none, in which case we (re)generate below to keep hit == miss.
        if [ "$cache_hit" -eq 1 ] \
            && ! find "$BUILD_DIR/provingKey" -name '*.exps.so' -print -quit | grep -q .; then
            info "Cache hit lacks .exps.so kernels, generating once (stale cache)"
            gen_exps_on_hit=1
        fi
    fi

    step "Building setup..."
    if [[ "$cache_hit" -eq 0 ]]; then
        info "Compiling zisk.pil..."
        run_compile_pil || return 1

        info "Generating pil-helpers..."
        run_pil_helpers || return 1

        info "Building proving key ($mode_label)..."
        local jobs_flags=()
        [[ -n "${RECURSIVE_JOBS}" ]] && jobs_flags+=(--recursive-jobs "${RECURSIVE_JOBS}")
        [[ -n "${SETUP_JOBS}" ]]     && jobs_flags+=(--setup-jobs "${SETUP_JOBS}")

        ensure rm -rf "$build_dir/provingKey" || return 1
        ensure cargo run --release --bin cargo-zisk-dev -- proofman-setup setup \
            --airout pil/zisk.pilout \
            --build-dir "$build_dir" \
            --fixed-dir tmp/fixed \
            --stark-structs state-machines/starkstructs.json \
            --hash "$HASH" \
            "${recursive_flag[@]}" \
            "${jobs_flags[@]}" || return 1

        run_gen_exps || return 1

        cache_proving_key "$build_dir" "$cache_entry" || return 1
    else
        info "Setup cache hit: $cache_entry, skipping build setup"

        if [[ "$gen_exps_on_hit" -eq 1 ]]; then
            info "Generating .exps.so kernels for cache hit (stale cache)"
            run_gen_exps || return 1

            cache_proving_key "$build_dir" "$cache_entry" || return 1
        fi
    fi

    if [[ "${INCLUDE_SNARK}" == "1" ]]; then
        step "Building snark setup..."
        run_setup_snark "$build_dir" || return 1
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
