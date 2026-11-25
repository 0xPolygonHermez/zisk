#!/usr/bin/env bash
#
# Run clippy on the workspace with a specific Rust version, using the same
# flags as the `lint` job in .github/workflows/pr.yml.
#
# The CI pins the clippy toolchain (CLIPPY_RUST_VERSION in pr.yml) so that a
# new stable release cannot break the lint job on its own. Before bumping that
# pin, run this script with the new version and fix what it reports:
#
#   tools/clippy.sh            # version pinned in CI (CLIPPY_RUST_VERSION)
#   tools/clippy.sh 1.101.0    # an explicit version
#   tools/clippy.sh stable     # whatever the latest stable is
#   tools/clippy.sh beta       # preview the lints of the next release
#
# Extra arguments after the version are passed to `cargo clippy`, e.g.
#   tools/clippy.sh 1.101.0 -p zisk-core
#
# The toolchain and its clippy component are installed with rustup if missing.
# Some crates need the ZisK guest toolchain (`cargo-zisk toolchain install`),
# as in CI.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$REPO_ROOT/.github/workflows/pr.yml"

usage() {
    sed -n '3,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    usage
    exit 0
fi

if [ $# -gt 0 ] && [ "${1#-}" = "$1" ]; then
    VERSION="$1"
    shift
else
    # Default: the version the CI lint job is pinned to.
    VERSION="$(sed -n 's/^[[:space:]]*CLIPPY_RUST_VERSION:[[:space:]]*"\{0,1\}\([^"[:space:]]*\)"\{0,1\}.*/\1/p' "$WORKFLOW" | head -n 1)"
    if [ -z "$VERSION" ]; then
        echo "error: could not read CLIPPY_RUST_VERSION from $WORKFLOW; pass a version explicitly" >&2
        exit 1
    fi
fi

if ! command -v rustup >/dev/null 2>&1; then
    echo "error: rustup is required to select the Rust version" >&2
    exit 1
fi

echo "==> Installing Rust $VERSION with clippy (no-op if already installed)"
rustup toolchain install "$VERSION" --profile minimal --component clippy --no-self-update

echo "==> $(rustup run "$VERSION" rustc --version)"
echo "==> $(rustup run "$VERSION" cargo clippy --version)"

cd "$REPO_ROOT"
echo "==> cargo +$VERSION clippy --all-features --all-targets $* -- -D warnings -A incomplete-features"
cargo "+$VERSION" clippy --all-features --all-targets "$@" -- -D warnings -A incomplete-features
