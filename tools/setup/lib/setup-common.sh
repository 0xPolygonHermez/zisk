#!/usr/bin/env bash
#
# Shared helpers for build-setup.sh. Sourced, not executed.
#
# Caller responsibilities before sourcing:
#   - set ROOT_DIR to the zisk repo root and cd there
#   - have cargo on PATH
#
# Variables this defines (read by callers):
#   PROOFMAN_DIR    resolved pil2-proofman checkout (env override honored)
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

# Resolve the pil2-proofman checkout. PROOFMAN_DIR env wins (handy for local dev
# against an unpushed branch, and required when proofman is a local path dep).
# Otherwise read the `proofman` git revision from Cargo.lock and locate cargo's
# checkout for it — that's the source actually compiled into cargo-zisk, so it's
# also the one whose package.json must feed the cache key.
resolve_proofman_dir() {
  cargo fetch >&2
  # Cargo.lock stores, for a git dep:
  #   source = "git+<url>?<query>#<full-sha>"
  # cargo checks the repo out under
  #   ~/.cargo/git/checkouts/pil2-proofman-<urlhash>/<short-sha>/
  # where <short-sha> is the first 7 chars of the rev. (No jq / cargo-metadata
  # JSON — coreutils only.)
  local src rev short dir
  src="$(awk '
    /^\[\[package\]\]/        { p=0 }
    /^name = "proofman"$/      { p=1 }
    p && /^source = "git\+/    { print; exit }
  ' "$ROOT_DIR/Cargo.lock")"
  if [ -z "$src" ]; then
    echo "proofman is not a git dependency in $ROOT_DIR/Cargo.lock — set PROOFMAN_DIR to the pil2-proofman checkout" >&2
    return 1
  fi
  rev="${src##*#}"   # strip everything up to the final '#'
  rev="${rev%\"}"    # strip the trailing quote
  short="${rev:0:7}"
  for dir in "$HOME"/.cargo/git/checkouts/pil2-proofman-*/"$short"; do
    if [ -f "$dir/package.json" ] && [ -d "$dir/pil2-components/lib/std/pil" ]; then
      printf '%s\n' "$dir"
      return 0
    fi
  done
  echo "could not locate the pil2-proofman checkout for rev $short under ~/.cargo/git/checkouts/ — set PROOFMAN_DIR to override" >&2
  return 1
}

if [ -n "${PROOFMAN_DIR:-}" ]; then
  [ -d "$PROOFMAN_DIR" ] || { echo "PROOFMAN_DIR not a directory: $PROOFMAN_DIR" >&2; exit 1; }
else
  PROOFMAN_DIR="$(resolve_proofman_dir)"
fi
[ -f "$PROOFMAN_DIR/package.json" ] || { echo "package.json not found at $PROOFMAN_DIR/package.json" >&2; exit 1; }
[ -d "$PROOFMAN_DIR/pil2-components/lib/std/pil" ] || { echo "pil2-components/lib/std/pil not found at $PROOFMAN_DIR" >&2; exit 1; }
echo "proofman dir: $PROOFMAN_DIR"

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

  # pil2-stark-setup is a transitive git dep, not a workspace member. Read its
  # source straight from Cargo.lock: for a git dep that's a stable
  #   source = "git+https://.../pil2-proofman.git?branch=X#<sha>"
  # — the same string on every machine (so the cache key is host-independent).
  # A local path dep has no `source` line, so this comes back empty and we abort
  # rather than fall back to a machine-specific path. (No jq / cargo-metadata.)
  pil2_stark_setup_source="$(awk '
    /^\[\[package\]\]/                { p=0 }
    /^name = "pil2-stark-setup"$/      { p=1 }
    p && /^source = /                  { sub(/^source = "/, ""); sub(/"$/, ""); print; exit }
  ' "$ROOT_DIR/Cargo.lock")"
  [ -n "$pil2_stark_setup_source" ] || \
    { echo "pil2-stark-setup has no git source in $ROOT_DIR/Cargo.lock — is it a local path dep? cache key would be machine-specific. aborting." >&2; exit 1; }

  echo "hashing $(wc -l < "$pil_list") .pil files + starkstructs.json + ${#fixed_bins[@]} *_fixed.bin + tool refs" >&2
  {
    xargs cat < "$pil_list"
    cat state-machines/starkstructs.json
    cat "${fixed_bins[@]}"
    printf 'pil2-compiler:%s\n' "$pil2_compiler_version"
    printf 'pil2-stark-setup:%s\n' "$pil2_stark_setup_source"
  } | sha256_hex
)
