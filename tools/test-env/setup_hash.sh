#!/bin/bash
#
# Print the ZisK setup input hash (the cache key) to stdout.
#
# Generates the frops fixed data (its output goes to stderr) and hashes the
# setup inputs: the .pil sources, the *_fixed.bin files and the
# pil2-compiler / pil2-stark-setup dependency refs. build_setup.sh calls this to
# derive its cache key (local_hash).
#
# Reuses PROOFMAN_DIR / ZISK_REPO_DIR from the environment when set (exported by
# build_setup.sh); otherwise resolves them on its own so it works standalone.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/utils.sh"

# Portable stdin -> lowercase hex digest (sha256sum on Linux, shasum on macOS/BSD).
sha256_hex() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum
    else shasum -a 256
    fi | awk '{print $1}'
}

# Generate the frops fixed data
generate_frops() {
    ensure cargo run --release --bin arith_frops_fixed_gen || return 1
    ensure cargo run --release --bin binary_basic_frops_fixed_gen || return 1
    ensure cargo run --release --bin binary_extension_frops_fixed_gen || return 1
}

# Return the hash for the local setup
compute_hash() (
    # Subshell so the EXIT trap cleans up the temp file without leaking into the caller.
    pil_list=$(mktemp)
    trap 'rm -f "$pil_list"' EXIT
    find pil state-machines precompiles -type f -name '*.pil' >> "$pil_list"
    find "$PROOFMAN_DIR/pil2-components/lib/std/pil" -type f -name '*.pil' >> "$pil_list"
    # LC_ALL=C: byte-ordered sort so the hash is locale-independent across machines.
    LC_ALL=C sort -o "$pil_list" "$pil_list"

    fixed_bins=(
        state-machines/arith/src/arith_frops_fixed.bin
        state-machines/binary/src/binary_basic_frops_fixed.bin
        state-machines/binary/src/binary_extension_frops_fixed.bin
    )
    for f in "${fixed_bins[@]}"; do
        [ -f "$f" ] || { echo "missing fixed binary: $f — run its generator first" >&2; exit 1; }
    done

    # The package.json "pil2-compiler" dependency value (e.g. ...pil2-compiler.git#v0.9.0).
    pil2_compiler_version="$(sed -nE 's/.*"pil2-compiler"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$PROOFMAN_DIR/package.json" | head -n1)"
    [ -n "$pil2_compiler_version" ] || \
        { echo "could not read \"pil2-compiler\" from $PROOFMAN_DIR/package.json" >&2; exit 1; }

    # pil2-stark-setup is a transitive dep. Prefer its Cargo.lock `source` (a git
    # dep gives a stable, host-independent string); a local path dep has no
    # `source` and is handled below. (coreutils only — no jq / cargo-metadata.)
    pil2_stark_setup_source="$(awk '
        /^\[\[package\]\]/                { p=0 }
        /^name = "pil2-stark-setup"$/      { p=1 }
        p && /^source = /                  { sub(/^source = "/, ""); sub(/"$/, ""); print; exit }
    ' "$ZISK_REPO/Cargo.lock")"
    if [ -z "$pil2_stark_setup_source" ]; then
        # Local path dep: derive the key from HEAD + working-tree state of the
        # crate, so local edits bust the cache instead of reusing a stale setup.
        stark_dir="$PROOFMAN_DIR/setup/pil2-stark"
        if git -C "$PROOFMAN_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
            head="$(git -C "$PROOFMAN_DIR" rev-parse HEAD 2>/dev/null)"
            wt="$( { git -C "$PROOFMAN_DIR" diff HEAD -- "$stark_dir";
                     git -C "$PROOFMAN_DIR" ls-files --others --exclude-standard -- "$stark_dir" \
                       | while IFS= read -r f; do printf '== %s ==\n' "$f"; cat "$PROOFMAN_DIR/$f"; done
                   } 2>/dev/null | sha256_hex )"
            pil2_stark_setup_source="local-path:$head:$wt"
        else
            wt="$(find "$stark_dir" -type f \( -name '*.rs' -o -name '*.toml' \) \
                | LC_ALL=C sort | xargs cat 2>/dev/null | sha256_hex)"
            pil2_stark_setup_source="local-path:$wt"
        fi
        info "pil2-stark-setup is a local path dep — using content-derived cache key ($pil2_stark_setup_source)" >&2
    fi

    info "hashing $(wc -l < "$pil_list") .pil files + starkstructs.json + ${#fixed_bins[@]} *_fixed.bin + tool refs" >&2
    {
        xargs cat < "$pil_list"
        cat state-machines/starkstructs.json
        cat "${fixed_bins[@]}"
        printf 'pil2-compiler:%s\n' "$pil2_compiler_version"
        printf 'pil2-stark-setup:%s\n' "$pil2_stark_setup_source"
    } | sha256_hex
)

main() {
    ZISK_REPO="$(get_zisk_repo_dir)"
    export ZISK_REPO_DIR="${ZISK_REPO}"
    cd "${ZISK_REPO}" || { echo "cannot cd to ${ZISK_REPO}" >&2; return 1; }

    PROOFMAN_DIR="${PROOFMAN_DIR:-$(resolve_proofman_dir)}" || return 1

    # frops output to stderr so stdout carries only the hash.
    generate_frops 1>&2 || return 1
    compute_hash
}

main
