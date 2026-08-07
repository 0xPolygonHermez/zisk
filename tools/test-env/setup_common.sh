#!/usr/bin/env bash
#
# Shared helpers for setup_build.sh. Sourced, not executed.
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
#   generate_fixed_data  cargo-run the fixed-column generators (honors SKIP_COMPILE_PIL)
#   compute_input_hash   print sha256 of the cache-key inputs to stdout
#
# Variables this reads (defaulted if unset):
#   SKIP_COMPILE_PIL     0|1 — when 1, generate_fixed_data is a no-op

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

# Print the pil2-compiler git URL#ref, or nothing when no branch is set. The
# branch is PIL2_COMPILER_BRANCH (set by --pil2-compiler-branch) if given, else
# gha_pil2_compiler_branch from the root Cargo.toml. Shared by the cache key and
# the installer so they can't disagree.
PIL2_COMPILER_REPO="https://github.com/0xPolygonHermez/pil2-compiler.git"
read_zisk_pil2_compiler_override() {
  local branch="${PIL2_COMPILER_BRANCH:-}"
  [ -n "$branch" ] || branch="$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
    | jq -r '.metadata.gha_pil2_compiler_branch // empty')"
  [ -n "$branch" ] || return 0
  printf '%s#%s\n' "$PIL2_COMPILER_REPO" "$branch"
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

# Fixed columns that a PIL loads from disk rather than building itself, either
# because an interpreted PIL loop would cost minutes of compile time (the
# jump_dest bitmap table) or because the data comes from a generator (frops).
# Required inputs to compile-pil and to the input-hash. Cheap to regenerate.
# Skipped under SKIP_COMPILE_PIL=1: the on-disk *_fixed.bin files are paired
# with the reused pilout, and generation is idempotent given unchanged
# sources, so regenerating only burns cargo-build time. compute_input_hash
# checks the bins exist and errors cleanly if they don't.
generate_fixed_data() {
  if [ "$SKIP_COMPILE_PIL" -eq 1 ]; then
    echo "==> generating fixed data (SKIPPED — reusing existing *_fixed.bin)"
    return
  fi
  echo "==> generating fixed data"
  cargo run --release --bin arith_frops_fixed_gen
  cargo run --release --bin binary_basic_frops_fixed_gen
  cargo run --release --bin binary_extension_frops_fixed_gen
  cargo run --release --bin jump_dest_bitmap_table_gen
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
    precompiles/evm/src/jump_dest_bitmap_table_fixed.bin
  )
  for f in "${fixed_bins[@]}"; do
    [ -f "$f" ] || { echo "missing fixed binary: $f — run its generator first" >&2; exit 1; }
  done

  # The pil2-compiler version that will actually compile the PIL. The override
  # wins over proofman's own package.json, so it must feed the cache key or two
  # builds with different overrides would collide. Falls back to proofman's value
  # when there is no override. Shared resolver with the installer.
  local pil2_compiler_version pil2_compiler_override
  pil2_compiler_override="$(read_zisk_pil2_compiler_override)"
  if [ -n "$pil2_compiler_override" ]; then
    pil2_compiler_version="$pil2_compiler_override"
  else
    pil2_compiler_version="$(sed -nE 's/.*"pil2-compiler"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$PROOFMAN_DIR/package.json" | head -n1)"
    [ -n "$pil2_compiler_version" ] || \
      { echo "could not read \"pil2-compiler\" from $PROOFMAN_DIR/package.json" >&2; exit 1; }
  fi

  # pil2-stark-setup is a transitive dep, not a workspace member. Key it by the
  # content (git tree OIDs + dirty state) of the proofman paths that feed setup
  # generation — not by the repo SHA from Cargo.lock, which rotates on every
  # proofman bump even when only prover runtime code changed. The setup crates'
  # library deps (common/ fields/ pilout/ util/) are deliberately not keyed; if
  # a proof fails after a bump that cache-hit here, rebuild once with
  # FORCE_SETUP_BUILD=1 and add the offending path below.
  local setup_tree_paths=(
    setup                  # pil2-stark-setup + stark-recurser + exps-codegen
    pil2-stark             # C++ library that computes the setup artifacts
    provers/starks-lib-c   # FFI wrapper + build.rs that drives the C++ build
  )
  local trees wt p
  if git -C "$PROOFMAN_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    trees="$(for p in "${setup_tree_paths[@]}"; do
      printf '%s:%s\n' "$p" \
        "$(git -C "$PROOFMAN_DIR" rev-parse -q --verify "HEAD:$p" 2>/dev/null || echo absent)"
    done)"
    # Uncommitted edits must bust the cache. Untracked files are filtered to
    # source extensions so build artifacts (libstarks.a lands inside
    # pil2-stark/) never leak into the key; `|| true` because an empty match
    # must not abort under pipefail.
    wt="$( { git -C "$PROOFMAN_DIR" diff HEAD -- "${setup_tree_paths[@]}";
             git -C "$PROOFMAN_DIR" ls-files --others --exclude-standard -- "${setup_tree_paths[@]}" \
               | { grep -E '\.(rs|toml|cpp|hpp|c|h|cu|cuh|asm|json|circom|ejs|js|sh|mk)$|(^|/)Makefile$' || true; } \
               | LC_ALL=C sort \
               | while IFS= read -r f; do printf '== %s ==\n' "$f"; cat "$PROOFMAN_DIR/$f" || true; done
           } 2>/dev/null | sha256_hex )"
    pil2_stark_setup_source="trees:$(printf '%s\nworktree:%s\n' "$trees" "$wt" | sha256_hex)"
  else
    # Not a git checkout — content-hash the source files of the same paths.
    # Same extension set as the untracked-file filter above, so a build input
    # that busts the key in a checkout busts it here too. Paths go into the
    # digest alongside contents, so a rename or a swap of two files is visible.
    wt="$(cd "$PROOFMAN_DIR" && find "${setup_tree_paths[@]}" -type f \
            \( -name '*.rs' -o -name '*.toml' -o -name '*.cpp' -o -name '*.hpp' \
               -o -name '*.c' -o -name '*.h' -o -name '*.cu' -o -name '*.cuh' \
               -o -name '*.asm' -o -name '*.json' -o -name '*.circom' -o -name '*.ejs' \
               -o -name '*.js' -o -name '*.sh' -o -name '*.mk' -o -name Makefile \) \
          2>/dev/null | LC_ALL=C sort \
          | while IFS= read -r f; do printf '== %s ==\n' "$f"; cat "$f" || true; done \
          | sha256_hex)"
    pil2_stark_setup_source="local-content:$wt"
  fi
  echo "pil2-stark-setup key: $pil2_stark_setup_source" >&2

  echo "hashing $(wc -l < "$pil_list") .pil files + starkstructs.json + ${#fixed_bins[@]} *_fixed.bin + tool refs" >&2
  {
    xargs cat < "$pil_list"
    cat state-machines/starkstructs.json
    cat "${fixed_bins[@]}"
    printf 'pil2-compiler:%s\n' "$pil2_compiler_version"
    printf 'pil2-stark-setup:%s\n' "$pil2_stark_setup_source"
  } | sha256_hex
)