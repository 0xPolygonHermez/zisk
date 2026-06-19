#!/usr/bin/env bash
#
# Shared helpers for build-setup.sh. Sourced, not executed.
#
# Caller responsibilities before sourcing:
#   - set ROOT_DIR to the zisk repo root and cd there
#   - have cargo on PATH
#
# Variables this defines (read by callers):
#   PROOFMAN_DIR    resolved pil2-proofman checkout
#   VERSION         zisk version from Cargo.toml
#   INCLUDE_PATHS   --include arg for compile-pil
#
# Functions this defines:
#   generate_frops       cargo-run the three frops generators (honors SKIP_COMPILE_PIL)
#   compute_input_hash   print sha256 of the cache-key inputs to stdout
#
# Variables this reads (defaulted if unset):
#   SKIP_COMPILE_PIL     0|1 — when 1, generate_frops is a no-op

: "${SKIP_COMPILE_PIL:=0}"

# Portable shims for utilities that ship as GNU-only on Linux but use different
# names on BSD userlands (macOS). Defined once so callers don't have to care.

# sha256_hex: read stdin, print lowercase hex digest, no trailing filename.
# Linux: sha256sum. macOS/BSD: shasum -a 256 (ships with the base system).
sha256_hex() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum
  else shasum -a 256
  fi | awk '{print $1}'
}

# Resolve the pil2-proofman checkout — always whatever cargo actually compiled
# into cargo-zisk, so this script can never drift from the build. `cargo metadata`
# reports proofman's on-disk manifest_path regardless of how it's depended on:
#   - git dep  => ~/.cargo/git/checkouts/pil2-proofman-<hash>/<short-sha>/proofman
#   - path dep => the local checkout, e.g. ../pil2-proofman/proofman
# That points at the `proofman` crate subdir; the checkout root (one level up)
# is what holds package.json and pil2-components, so strip the crate segment.
resolve_proofman_dir() {
  cargo fetch >&2
  local manifest root
  manifest="$(cargo metadata --format-version 1 2>/dev/null \
    | jq -r '.packages[] | select(.name=="proofman") | .manifest_path')"
  if [ -z "$manifest" ] || [ "$manifest" = "null" ]; then
    echo "cargo metadata did not report a 'proofman' package — is it in the dependency tree?" >&2
    return 1
  fi
  root="$(cd "${manifest%/Cargo.toml}/.." && pwd)"
  if [ -f "$root/package.json" ] && [ -d "$root/pil2-components/lib/std/pil" ]; then
    printf '%s\n' "$root"
    return 0
  fi
  echo "proofman manifest '$manifest' does not resolve to a pil2-proofman checkout ($root)" >&2
  return 1
}

PROOFMAN_DIR="$(resolve_proofman_dir)" || exit 1
echo "proofman dir: $PROOFMAN_DIR" >&2

VERSION="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$ROOT_DIR/Cargo.toml")"
INCLUDE_PATHS="pil,${PROOFMAN_DIR}/pil2-components/lib/std/pil,state-machines,precompiles"

# Required inputs to compile-pil and to the input-hash. Cheap to regenerate.
# Skipped under SKIP_COMPILE_PIL=1: the on-disk *_fixed.bin files are paired
# with the reused pilout, and frops generation is idempotent given unchanged
# sources, so regenerating only burns cargo-build time. compute_input_hash
# checks the bins exist and errors cleanly if they don't.
generate_frops() {
  if [ "$SKIP_COMPILE_PIL" -eq 1 ]; then
    echo "==> generating frops fixed data (SKIPPED — reusing existing *_fixed.bin)"
    return
  fi
  echo "==> generating frops fixed data"
  cargo run --release --bin arith_frops_fixed_gen
  cargo run --release --bin binary_basic_frops_fixed_gen
  cargo run --release --bin binary_extension_frops_fixed_gen
}

compute_input_hash() (
  # Subshell: EXIT trap cleans up the temp file on every return path
  # (success and the early-error returns) without leaking a RETURN trap
  # into the calling shell.
  pil_list=$(mktemp)
  trap 'rm -f "$pil_list"' EXIT
  find pil state-machines precompiles -type f -name '*.pil' >> "$pil_list"
  find "$PROOFMAN_DIR/pil2-components/lib/std/pil" -type f -name '*.pil' >> "$pil_list"
  # LC_ALL=C: byte-ordered sort so the hash matches across machines regardless
  # of locale (en_US.UTF-8 vs C can reorder paths with punctuation).
  LC_ALL=C sort -o "$pil_list" "$pil_list"

  fixed_bins=(
    state-machines/arith/src/arith_frops_fixed.bin
    state-machines/binary/src/binary_basic_frops_fixed.bin
    state-machines/binary/src/binary_extension_frops_fixed.bin
  )
  for f in "${fixed_bins[@]}"; do
    [ -f "$f" ] || { echo "missing fixed binary: $f — run its generator first" >&2; exit 1; }
  done

  # The package.json dependency value, e.g.
  #   "pil2-compiler": "https://github.com/.../pil2-compiler.git#v0.9.0"
  # Extracted with sed (no jq); identical to what jq -r '.dependencies."pil2-compiler"'
  # returned, so the cache key is unchanged.
  pil2_compiler_version="$(sed -nE 's/.*"pil2-compiler"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$PROOFMAN_DIR/package.json" | head -n1)"
  [ -n "$pil2_compiler_version" ] || \
    { echo "could not read \"pil2-compiler\" from $PROOFMAN_DIR/package.json" >&2; exit 1; }

  # pil2-stark-setup is a transitive dep, not a workspace member. Prefer its
  # source straight from Cargo.lock: for a git dep that's a stable
  #   source = "git+https://.../pil2-proofman.git?branch=X#<sha>"
  # — the same string on every machine (so the cache key is host-independent).
  # A local path dep has no `source` line; that case is handled below.
  # (No jq / cargo-metadata — coreutils only.)
  pil2_stark_setup_source="$(awk '
    /^\[\[package\]\]/                { p=0 }
    /^name = "pil2-stark-setup"$/      { p=1 }
    p && /^source = /                  { sub(/^source = "/, ""); sub(/"$/, ""); print; exit }
  ' "$ROOT_DIR/Cargo.lock")"
  if [ -z "$pil2_stark_setup_source" ]; then
    # No `source` line => local path dep (local dev). The key is necessarily
    # machine-specific here, which is correct: on a local checkout you're editing
    # proofman, so the key MUST track those edits or a stale setup gets reused.
    # Derive it from the checkout's HEAD plus the working-tree state of the
    # pil2-stark-setup crate, so uncommitted edits bust the cache.
    local stark_dir head wt
    stark_dir="$PROOFMAN_DIR/setup/pil2-stark"
    if git -C "$PROOFMAN_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
      head="$(git -C "$PROOFMAN_DIR" rev-parse HEAD 2>/dev/null)"
      # Hash tracked-but-modified + untracked files under the crate dir. Empty
      # (clean tree) => stable hash of "", so a clean checkout keys off HEAD alone.
      wt="$( { git -C "$PROOFMAN_DIR" diff HEAD -- "$stark_dir";
               git -C "$PROOFMAN_DIR" ls-files --others --exclude-standard -- "$stark_dir" \
                 | while IFS= read -r f; do printf '== %s ==\n' "$f"; cat "$PROOFMAN_DIR/$f"; done
             } 2>/dev/null | sha256_hex )"
      pil2_stark_setup_source="local-path:$head:$wt"
    else
      # Not a git checkout — hash the crate's source tree contents. cat the files
      # in sorted order through one digest so the result is path-stable.
      wt="$(find "$stark_dir" -type f \( -name '*.rs' -o -name '*.toml' \) \
        | LC_ALL=C sort | xargs cat 2>/dev/null | sha256_hex)"
      pil2_stark_setup_source="local-path:$wt"
    fi
    echo "pil2-stark-setup is a local path dep — using content-derived cache key ($pil2_stark_setup_source)" >&2
  fi

  echo "hashing $(wc -l < "$pil_list") .pil files + starkstructs.json + ${#fixed_bins[@]} *_fixed.bin + tool refs" >&2
  {
    xargs cat < "$pil_list"
    cat state-machines/starkstructs.json
    cat "${fixed_bins[@]}"
    printf 'pil2-compiler:%s\n' "$pil2_compiler_version"
    printf 'pil2-stark-setup:%s\n' "$pil2_stark_setup_source"
  } | sha256_hex
)
