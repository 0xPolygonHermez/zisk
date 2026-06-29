#!/usr/bin/env bash
#
# Run `cargo machete` (unused-dependency detector) across every crate in the
# repository — workspace members, the excluded `test-artifacts`, and the
# `examples/*` sub-workspaces. `cargo machete` walks the directory tree for
# every Cargo.toml, so a single root invocation already covers all crates; this
# script just wraps install + flags + a clear summary.
#
# Usage:
#   scripts/cargo-machete-all.sh            # fast source-scan over the whole tree
#   scripts/cargo-machete-all.sh --fix      # auto-remove unused deps it is sure about
#   WITH_METADATA=1 scripts/cargo-machete-all.sh   # cargo-metadata-backed (slower, fewer false positives)
#
# Exit code: 0 = no unused deps found, non-zero = unused deps reported (CI-friendly).

set -euo pipefail

# Resolve repo root from this script's location, independent of CWD.
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"

# Ensure cargo-machete is available; install pinned if not.
if ! cargo machete --help >/dev/null 2>&1; then
  echo ">> cargo-machete not found; installing (cargo install cargo-machete)..." >&2
  cargo install cargo-machete --locked
fi

echo ">> cargo machete: $(cargo machete --version 2>/dev/null || echo '?')" >&2
echo ">> scanning all crates under: ${REPO_ROOT}" >&2

# Assemble flags.
ARGS=()
[[ "${WITH_METADATA:-0}" == "1" ]] && ARGS+=(--with-metadata)
# Pass through user flags (e.g. --fix).
ARGS+=("$@")

# Single recursive scan from the repo root covers every Cargo.toml in the tree.
cd "${REPO_ROOT}"
cargo machete "${ARGS[@]}"
