#!/usr/bin/env bash
#
# Shared helpers for build-setup.sh and fetch-setup.sh. Sourced, not executed.
#
# Caller responsibilities before sourcing:
#   - set ROOT_DIR to the zisk repo root and cd there
#   - have jq and cargo on PATH (curl too, if the caller will hit the bucket)
#
# Variables this defines (read by callers):
#   PROOFMAN_DIR    resolved pil2-proofman checkout (env override honored)
#   VERSION         zisk version from Cargo.toml
#   BUCKET          public GCS setup bucket URL
#   PK_NAME         proving-key tarball filename for VERSION
#   HASH_NAME       input-hash sidecar filename for VERSION
#   INCLUDE_PATHS   --include arg for compile-pil
#
# Functions this defines:
#   generate_frops       cargo-run the three frops generators (honors SKIP_COMPILE_PIL)
#   compute_input_hash   print sha256 of the cache-key inputs to stdout
#
# Variables this reads (defaulted if unset):
#   SKIP_COMPILE_PIL     0|1 — when 1, generate_frops is a no-op
#   RELEASE              0|1 — when 0 (default), PK_NAME / HASH_NAME use the
#                        "pre-" prefix that matches package-proving-key.sh's
#                        default upload namespace. Set to 1 to look up the
#                        un-prefixed release artifacts.

: "${SKIP_COMPILE_PIL:=0}"
: "${RELEASE:=0}"

# Portable shims for utilities that ship as GNU-only on Linux but use different
# names on BSD userlands (macOS). Defined once so callers don't have to care.

# sha256_hex: read stdin, print lowercase hex digest, no trailing filename.
# Linux: sha256sum. macOS/BSD: shasum -a 256 (ships with the base system).
sha256_hex() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum
  else shasum -a 256
  fi | awk '{print $1}'
}

# mktemp_tarball: create a unique temp path ending in .tar.gz, print it.
# GNU mktemp has --suffix; BSD mktemp does not. Two calls + mv is the portable
# spelling that works on both.
mktemp_tarball() {
  local t
  t="$(mktemp)"
  mv "$t" "$t.tar.gz"
  printf '%s\n' "$t.tar.gz"
}

# Resolve the pil2-proofman checkout. PROOFMAN_DIR env wins (handy for local dev
# against an unpushed branch). Otherwise read it from cargo's git checkout for
# the `proofman` dep in Cargo.toml — that's the source that will actually be
# compiled into cargo-zisk, so it's also the one whose pil/package.json must
# feed the cache key.
resolve_proofman_dir() {
  cargo fetch >&2
  local manifest
  manifest="$(cargo metadata --format-version 1 \
    | jq -r '.packages[] | select(.name=="proofman") | .manifest_path' | head -n1)"
  [ -n "$manifest" ] && [ "$manifest" != "null" ] \
    || { echo "could not locate 'proofman' crate via cargo metadata" >&2; return 1; }
  local dir
  dir="$(dirname "$manifest")"
  while [ "$dir" != "/" ] && [ -n "$dir" ]; do
    if [ -f "$dir/package.json" ] && [ -d "$dir/pil2-components/lib/std/pil" ]; then
      printf '%s\n' "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  echo "walked up from $manifest without finding package.json + pil2-components/" >&2
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
# Mirror package-proving-key.sh: non-release uploads land at "pre-<VERSION>",
# release uploads land at the bare "<VERSION>". Keep the lookup symmetric.
if [ "$RELEASE" -ne 1 ]; then
  VERSION="pre-${VERSION}"
fi
BUCKET="https://storage.googleapis.com/zisk-setup"
INCLUDE_PATHS="pil,${PROOFMAN_DIR}/pil2-components/lib/std/pil,state-machines,precompiles"
PK_NAME="zisk-provingkey-${VERSION}.tar.gz"
HASH_NAME="zisk-provingkey-${VERSION}.input-hash"

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

  pil2_compiler_version="$(jq -r '.dependencies."pil2-compiler"' "$PROOFMAN_DIR/package.json")"
  [ -n "$pil2_compiler_version" ] && [ "$pil2_compiler_version" != "null" ] || \
    { echo "could not read .dependencies.pil2-compiler from $PROOFMAN_DIR/package.json" >&2; exit 1; }

  # No --no-deps: pil2-stark-setup is a transitive git dep, not a workspace
  # member, so --no-deps would silently drop it and the hash would never
  # invalidate on upstream setup-lib changes.
  #
  # Require .source explicitly: for a git/registry dep, .source is a stable
  # URL+rev string (e.g. "git+https://.../pil2-proofman.git?branch=X#<sha>")
  # that's the same on every machine. .manifest_path would be the local
  # cargo checkout dir (~/.cargo/git/checkouts/...), which differs per host
  # and would break cross-machine cache lookups. Fail loudly instead.
  pil2_stark_setup_source="$(cargo metadata --format-version 1 \
    | jq -r '.packages[]|select(.name=="pil2-stark-setup")|.source')"
  [ -n "$pil2_stark_setup_source" ] && [ "$pil2_stark_setup_source" != "null" ] || \
    { echo "pil2-stark-setup has no .source in cargo metadata — is it a local path dep? cache key would be machine-specific. aborting." >&2; exit 1; }

  echo "hashing $(wc -l < "$pil_list") .pil files + starkstructs.json + ${#fixed_bins[@]} *_fixed.bin + tool refs" >&2
  {
    xargs cat < "$pil_list"
    cat state-machines/starkstructs.json
    cat "${fixed_bins[@]}"
    printf 'pil2-compiler:%s\n' "$pil2_compiler_version"
    printf 'pil2-stark-setup:%s\n' "$pil2_stark_setup_source"
  } | sha256_hex
)
