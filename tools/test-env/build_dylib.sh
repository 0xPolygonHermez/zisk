#!/bin/bash
#
# Rebuild the macOS witness libraries and collect the resulting *.dylib into
# build/dylib (preserving the provingKey/ tree-relative paths).

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

    step "Rebuilding dylib files..."
    # Rebuild only the regular provingKey witness libs.
    ensure cargo-zisk-dev proofman-setup rebuild-witness-libs \
        --proving-key "$build_dir/provingKey" -j 1 || return 1

    step "Collecting dylibs to $build_dir/dylib..."
    local dest
    dest="$(pwd)/$build_dir/dylib"
    ensure rm -rf "$dest" || return 1

    ( cd "$build_dir" || exit 1
      find provingKey -type f -name '*.dylib' -print0 \
        | while IFS= read -r -d '' f; do
            mkdir -p "$dest/$(dirname "$f")"
            cp "$f" "$dest/$f"
          done
    ) || return 1

    local count
    count="$(find "$build_dir/dylib" -name '*.dylib' | wc -l | tr -d ' ')"
    info "Collected $count dylib(s)"
    [ "$count" -gt 0 ] || { err "rebuild produced no .dylib"; return 1; }

    success "dylib files built and collected into $build_dir/dylib"
}

main
