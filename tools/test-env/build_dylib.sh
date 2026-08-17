#!/bin/bash
#
# Rebuild the macOS witness libraries — provingKey/ and, when present,
# provingKeySnark/ — and collect the resulting *.dylib into build/dylib,
# preserving the tree-relative paths.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/utils.sh"

main() {
    info "▶️  Running $(basename "$0") script..."

    [[ "$(uname -s)" == "Darwin" ]] || { err "build_dylib.sh must run on macOS (darwin)" true; return 1; }

    local build_dir="build"
    current_step=1
    total_steps=2

    ZISK_REPO="$(get_zisk_repo_dir)"
    export ZISK_REPO_DIR="${ZISK_REPO}"
    ensure cd "${ZISK_REPO}" || return 1

    [[ -d "$build_dir/provingKey" ]] || { err "$build_dir/provingKey not found" true; return 1; }

    # Sourced for export_proofman_paths: rebuild-witness-libs compiles the stored
    # .cpp against CIRCOM_HELPERS_DIR (which holds the Makefile) + GOLDILOCKS_SRC_DIR.
    ROOT_DIR="${ZISK_REPO}"
    source "${SCRIPT_DIR}/setup_common.sh" || return 1

    # provingKeySnark/ holds recursivef (built with setup/circom, which has a Darwin
    # branch) and final (setup/final_snark_circom, Linux-only: g++ -flarge-source-files,
    # nasm -f elf64). rebuild-witness-libs takes the whole tree, so while final can't
    # build here we pass one exposing recursivef alone — a symlink, so its .dylib still
    # lands in the real tree (the builder canonicalizes the output dir).
    local snark_dir="$build_dir/provingKeySnark"
    local snark_recursivef_only="$build_dir/.provingKeySnark-darwin"
    local snark_args=()
    if [[ -d "$snark_dir" ]]; then
        if grep -q Darwin "${FINAL_SNARK_CIRCOM_HELPERS_DIR}/Makefile" 2>/dev/null; then
            snark_args=(--proving-key-snark "$snark_dir")   # Darwin branch: build both
        elif [[ -f "$snark_dir/recursivef/recursivef.cpp" ]]; then
            warn "final_snark_circom's Makefile is Linux-only: rebuilding recursivef, skipping final"
            ensure rm -rf "$snark_recursivef_only" || return 1
            ensure mkdir -p "$snark_recursivef_only" || return 1
            ensure ln -s "$(cd "$snark_dir/recursivef" && pwd)" \
                "$snark_recursivef_only/recursivef" || return 1
            snark_args=(--proving-key-snark "$snark_recursivef_only")
        else
            warn "$snark_dir has no recursivef/recursivef.cpp — skipping the snark witness libs"
        fi
    fi

    step "Rebuilding dylib files..."
    ensure cargo-zisk-dev proofman-setup rebuild-witness-libs \
        --proving-key "$build_dir/provingKey" \
        ${snark_args[@]+"${snark_args[@]}"} \
        -j 1 || { rm -rf "$snark_recursivef_only"; return 1; }
    rm -rf "$snark_recursivef_only"

    step "Collecting dylibs to $build_dir/dylib..."
    local dest
    dest="$(pwd)/$build_dir/dylib"
    ensure rm -rf "$dest" || return 1

    # Keep the tree prefixes: upload_setup.sh merges each into the build/ tree of
    # the same name (SETUP_ADD_DYLIBS).
    ( cd "$build_dir" || exit 1
      for tree in provingKey provingKeySnark; do
          [ -d "$tree" ] || continue
          find "$tree" -type f -name '*.dylib' -print0 \
            | while IFS= read -r -d '' f; do
                mkdir -p "$dest/$(dirname "$f")"
                cp "$f" "$dest/$f"
              done
      done
    ) || return 1

    local count
    count="$(find "$build_dir/dylib" -name '*.dylib' | wc -l | tr -d ' ')"
    info "Collected $count dylib(s)"
    [ "$count" -gt 0 ] || { err "rebuild produced no .dylib"; return 1; }

    success "dylib files built and collected into $build_dir/dylib"
}

main
