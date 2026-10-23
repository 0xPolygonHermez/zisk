#!/bin/bash
#
# Run `cargo update` in every workspace of this repo that depends, directly or
# indirectly, on a crate from pil2-proofman.
#
# A crate is a proofman crate when its `repository` or its `source` points to
# github.com/0xPolygonHermez/pil2-proofman. This works the same when the
# dependency is taken from git, from a local path or from crates.io.
#
# The resolved graph of each workspace is read with `cargo metadata --frozen`
# (no network, Cargo.lock is not touched). When Cargo.lock is missing or out of
# date, `cargo metadata` runs again without --frozen, which resolves the
# dependencies and rewrites Cargo.lock.
#
# Usage:
#   update-proofman-deps.sh [--list] [--] [cargo update args...]
#
#   --list     Only print the workspace directories that would be updated.
#   Any remaining arguments are passed to `cargo update`
#   (e.g. `--dry-run`, `-p proofman`).

set -euo pipefail

PROOFMAN_REPO="github.com/0xPolygonHermez/pil2-proofman"

if [ -t 1 ]; then
    BOLD=$(tput bold); GREEN=$(tput setaf 2); RED=$(tput setaf 1); YELLOW=$(tput setaf 3); RESET=$(tput sgr0)
else
    BOLD=""; GREEN=""; RED=""; YELLOW=""; RESET=""
fi

info() { echo "${BOLD}${GREEN}▶ $1${RESET}"; }
warn() { echo "${BOLD}${YELLOW}🚨  $1${RESET}" >&2; }
err()  { echo "${RED}❌ Error: $1${RESET}" >&2; }

LIST_ONLY=0
while [ $# -gt 0 ]; do
    case "$1" in
        --list) LIST_ONLY=1; shift ;;
        --) shift; break ;;
        -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) break ;;
    esac
done
CARGO_UPDATE_ARGS=("$@")

for cmd in cargo jq; do
    command -v "$cmd" >/dev/null 2>&1 || { err "'$cmd' is required"; exit 1; }
done

REPO_ROOT=$(git -C "$(dirname "$0")" rev-parse --show-toplevel 2>/dev/null) || REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$REPO_ROOT"

relpath() {
    local rel=${1#"$REPO_ROOT"}; rel=${rel#/}; echo "${rel:-.}"
}

# Print the proofman crates found in the resolved graph of a workspace root manifest.
proofman_in_workspace() {
    local root_manifest="$1" metadata
    if ! metadata=$(cargo metadata --frozen --format-version 1 --manifest-path "$root_manifest" 2>/dev/null); then
        warn "$(relpath "$(dirname "$root_manifest")"): Cargo.lock missing or out of date, resolving dependencies"
        metadata=$(cargo metadata --format-version 1 --manifest-path "$root_manifest") || return 1
    fi
    echo "$metadata" | jq -r --arg repo "$PROOFMAN_REPO" \
        '[.packages[] | select(((.repository // "") | contains($repo)) or ((.source // "") | contains($repo))) | .name] | unique | .[]'
}

info "Locating workspace roots"
WORKSPACE_ROOTS=$(find . -name Cargo.toml -not -path '*/target/*' -not -path '*/.git/*' | sort | while read -r manifest; do
    cargo locate-project --workspace --message-format plain --manifest-path "$manifest" 2>/dev/null || true
done | sort -u)

TO_UPDATE=()
SKIPPED=()
while read -r root_manifest; do
    [ -n "$root_manifest" ] || continue
    dir=$(dirname "$root_manifest")
    rel=$(relpath "$dir")
    if ! found=$(proofman_in_workspace "$root_manifest" | tr '\n' ' '); then
        warn "cannot resolve $rel, skipping"
        SKIPPED+=("$rel")
        continue
    fi
    if [ -n "$found" ]; then
        echo "   ${GREEN}✔${RESET} $rel  ${YELLOW}[$found]${RESET}"
        TO_UPDATE+=("$dir")
    else
        echo "   - $rel"
    fi
done <<< "$WORKSPACE_ROOTS"

if [ ${#TO_UPDATE[@]} -eq 0 ]; then
    warn "no workspace depends on proofman crates"
    exit 0
fi

if [ "$LIST_ONLY" -eq 1 ]; then
    info "Workspaces depending on proofman crates:"
    for dir in "${TO_UPDATE[@]}"; do echo "   $(relpath "$dir")"; done
    exit 0
fi

FAILED=()
for dir in "${TO_UPDATE[@]}"; do
    rel=$(relpath "$dir")
    info "cargo update in $rel ${CARGO_UPDATE_ARGS[*]:-}"
    if ! (cd "$dir" && cargo update "${CARGO_UPDATE_ARGS[@]+"${CARGO_UPDATE_ARGS[@]}"}"); then
        err "cargo update failed in $rel"
        FAILED+=("$rel")
    fi
done

echo
info "Summary: ${#TO_UPDATE[@]} workspace(s) processed, ${#FAILED[@]} failed, ${#SKIPPED[@]} skipped"
for rel in "${SKIPPED[@]+"${SKIPPED[@]}"}"; do echo "   skipped: $rel"; done
for rel in "${FAILED[@]+"${FAILED[@]}"}";  do echo "   failed:  $rel"; done
[ ${#FAILED[@]} -eq 0 ]
