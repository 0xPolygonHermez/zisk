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
# Variables this exports (read by the setup binaries, not by callers):
#   CIRCOM_HELPERS_DIR, FINAL_SNARK_CIRCOM_HELPERS_DIR, CIRCUITS_GL_PATH,
#   CIRCUITS_BN128_PATH, RECURSER_CIRCUITS_PATH,
#   RECURSER_CIRCUITS_COMPRESSED_FINAL_PATH, RECURSER_PIL_PATH, STD_PIL_PATH,
#   GOLDILOCKS_SRC_DIR — all under $PROOFMAN_DIR. See export_proofman_paths.
#
# Functions this defines:
#   generate_fixed_data  cargo-run the fixed-column generators (honors SKIP_COMPILE_PIL)
#   compute_input_hash   print sha256 of the cache-key inputs to stdout
#
# Variables this reads (defaulted if unset):
#   SKIP_COMPILE_PIL         0|1 — when 1, generate_fixed_data is a no-op
#   ZISK_PROOFMAN_CACHE_DIR  where crates.io-mode pil2-proofman checkouts are
#                            fetched (default ~/.zisk/pil2-proofman)

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

PIL2_PROOFMAN_REPO="https://github.com/0xPolygonHermez/pil2-proofman.git"
# Registry-mode checkouts land here, keyed by commit so versions never clash.
: "${ZISK_PROOFMAN_CACHE_DIR:=$HOME/.zisk/pil2-proofman}"

# Whether $1 is a pil2-proofman checkout root: it must carry the two things the
# setup needs beyond the Rust crates — the pil2-stark package.json (pil2-compiler
# version) and the std PIL library (compile-pil's include path).
is_proofman_checkout() {
  [ -f "$1/setup/pil2-stark/package.json" ] && [ -d "$1/pil2-components/lib/std/pil" ]
}

# Fetch the checkout a crates.io dependency was published from and print its
# root: the setup needs repo content no crate packages (the std PIL library, and
# proofman-cli, whose crate is `publish = false`). The commit isn't guessed —
# cargo records it in each crate's .cargo_vcs_info.json, so the checkout matches
# the compiled code exactly.
#
# $1 = the crate's registry directory, $2 = repository URL from cargo metadata.
fetch_proofman_checkout() {
  local crate_dir="$1" repo="$2"
  local vcs="$crate_dir/.cargo_vcs_info.json"
  local sha dir tmp

  if [ ! -f "$vcs" ]; then
    echo "no .cargo_vcs_info.json in $crate_dir — that crate was published without git info, so the matching pil2-proofman commit is unknown" >&2
    return 1
  fi
  sha="$(jq -r '.git.sha1 // empty' "$vcs")"
  [ -n "$sha" ] || { echo "no git.sha1 in $vcs" >&2; return 1; }
  case "$repo" in ""|null) repo="$PIL2_PROOFMAN_REPO" ;; esac

  dir="$ZISK_PROOFMAN_CACHE_DIR/$sha"
  # Written last, so an interrupted fetch is never reused. Lives under .git/ to
  # stay out of the worktree, which compute_input_hash scans for dirty state.
  if [ -f "$dir/.git/zisk_fetch_ok" ]; then
    printf '%s\n' "$dir"
    return 0
  fi

  echo "==> fetching pil2-proofman $sha (the commit the crates.io deps were published from)" >&2
  mkdir -p "$ZISK_PROOFMAN_CACHE_DIR"
  rm -rf "$dir"
  tmp="$(mktemp -d "$ZISK_PROOFMAN_CACHE_DIR/.tmp.XXXXXX")"
  (
    set -e
    git init -q "$tmp"
    git -C "$tmp" remote add origin "$repo"
    # GitHub serves reachable SHAs directly, so the common path stays shallow.
    # Fall back to a full fetch on servers that refuse a by-SHA request.
    git -C "$tmp" fetch -q --depth 1 origin "$sha" || git -C "$tmp" fetch -q origin
    git -C "$tmp" checkout -q --detach "$sha"
  ) >&2 || { rm -rf "$tmp"; echo "failed to fetch pil2-proofman $sha from $repo" >&2; return 1; }
  touch "$tmp/.git/zisk_fetch_ok"

  # Publish under the final name. A racing job's tree is identical, so keep it
  # and drop ours (plain `mv` onto an existing dir would nest, hence the check).
  if [ -d "$dir" ]; then rm -rf "$tmp"; else mv "$tmp" "$dir"; fi
  printf '%s\n' "$dir"
}

# Resolve the pil2-proofman checkout — always whatever cargo actually compiled
# into cargo-zisk, so this script can never drift from the build. `cargo metadata`
# reports proofman's manifest_path and source however it's depended on:
#   - path/git dep => <checkout>/proofman/Cargo.toml, so the root is one level up
#     (that's what holds package.json and pil2-components)
#   - registry dep => that one crate and no checkout, so fetch its source commit
resolve_proofman_dir() {
  cargo fetch >&2
  local meta manifest source repo root
  meta="$(cargo metadata --format-version 1 2>/dev/null)"
  manifest="$(printf '%s' "$meta" | jq -r '.packages[] | select(.name=="proofman") | .manifest_path')"
  if [ -z "$manifest" ] || [ "$manifest" = "null" ]; then
    echo "cargo metadata did not report a 'proofman' package — is it in the dependency tree?" >&2
    return 1
  fi
  source="$(printf '%s' "$meta" | jq -r '.packages[] | select(.name=="proofman") | .source')"
  repo="$(printf '%s' "$meta" | jq -r '.packages[] | select(.name=="proofman") | .repository')"

  case "$source" in
    registry+*) root="$(fetch_proofman_checkout "${manifest%/Cargo.toml}" "$repo")" || return 1 ;;
    *)          root="$(cd "${manifest%/Cargo.toml}/.." && pwd)" ;;
  esac

  if is_proofman_checkout "$root"; then
    printf '%s\n' "$root"
    return 0
  fi
  echo "proofman manifest '$manifest' does not resolve to a pil2-proofman checkout ($root)" >&2
  return 1
}

PROOFMAN_DIR="$(resolve_proofman_dir)" || exit 1
echo "proofman dir: $PROOFMAN_DIR" >&2

# Point the setup binaries at the repo content they need (the circom binary, the
# recurser circuits/PIL, the std PIL library, the goldilocks headers). Each has a
# compile-time-baked default of <CARGO_MANIFEST_DIR>/../.., which is the proofman
# repo root only while proofman is a git/path dep. For a crates.io dep it lands in
# ~/.cargo/registry/src, where none of it exists — cargo cannot package files
# outside a crate directory, and these all sit outside setup/pil2-stark. Exporting
# them from the resolved checkout makes both dependency modes behave identically.
#
# Fatal, not best-effort: PROOFMAN_DIR already passed is_proofman_checkout, so a
# missing path here means the checkout is broken and nothing downstream can work.
# The binaries would still "resolve" it — resolve_path_env returns an env value
# verbatim without an existence check, and resolve_circom_exec silently falls back
# to a bare `circom` on PATH — turning this into an ENOENT tens of minutes deep
# into the setup instead of here.
export_proofman_paths() {
  local cv="$PROOFMAN_DIR/setup/stark-recurser/stark2circom/circom_verifier"
  local entry var path missing=0

  # var:path-under-checkout. RECURSER_CIRCUITS_COMPRESSED_FINAL_PATH deliberately
  # points at helper_circuits, NOT circuits.bn128: bn128 shadows
  # circuits.gl/merkle.circom and drags in circomlib's comparators.circom, which
  # isn't on the include path (see the comment in proofman's recursive_setup.rs).
  local -a spec=(
    "CIRCOM_HELPERS_DIR:$PROOFMAN_DIR/setup/circom"
    "FINAL_SNARK_CIRCOM_HELPERS_DIR:$PROOFMAN_DIR/setup/final_snark_circom"
    "CIRCUITS_GL_PATH:$cv/circuits.gl"
    "CIRCUITS_BN128_PATH:$cv/circuits.bn128"
    "RECURSER_CIRCUITS_PATH:$cv/helper_circuits"
    "RECURSER_CIRCUITS_COMPRESSED_FINAL_PATH:$cv/helper_circuits"
    "RECURSER_PIL_PATH:$PROOFMAN_DIR/setup/stark-recurser/plonk2pil/pil"
    "STD_PIL_PATH:$PROOFMAN_DIR/pil2-components/lib/std/pil"
    "GOLDILOCKS_SRC_DIR:$PROOFMAN_DIR/pil2-stark/src/goldilocks/src"
  )

  for entry in "${spec[@]}"; do
    var="${entry%%:*}"
    path="${entry#*:}"
    # An already-set value wins, matching resolve_path_env's own precedence (it
    # checks the env var before anything else), so a caller can still override.
    if [ -n "${!var:-}" ]; then
      echo "$var: ${!var} (from environment)" >&2
      export "$var"
      continue
    fi
    if [ ! -e "$path" ]; then
      echo "error: $var would point at a missing path: $path" >&2
      missing=1
      continue
    fi
    export "$var=$path"
  done

  if [ "$missing" -eq 1 ]; then
    echo "the pil2-proofman checkout at $PROOFMAN_DIR is missing setup assets — it is incomplete for a setup build" >&2
    exit 1
  fi

  # Trace the circom binary the setup will actually exec, so a missing one is a
  # startup error rather than a mid-setup ENOENT. Mirrors resolve_circom_exec's
  # OS split (circom on Linux, circom_mac on Darwin); anything else there would
  # false-alarm on macOS while the run is in fact fine.
  local circom_bin=circom
  [ "$(uname -s)" = "Darwin" ] && circom_bin=circom_mac
  local circom_path="$CIRCOM_HELPERS_DIR/$circom_bin"
  if [ ! -x "$circom_path" ]; then
    echo "error: circom binary not found or not executable: $circom_path" >&2
    exit 1
  fi
  echo "circom: $circom_path" >&2
}
export_proofman_paths

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
  cargo run --release --bin zisk-arith-frops-fixed-gen
  cargo run --release --bin zisk-binary-basic-frops-fixed-gen
  cargo run --release --bin zisk-binary-extension-frops-fixed-gen
  cargo run --release --bin jump-dest-bitmap-table-gen
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
    pil2_compiler_version="$(sed -nE 's/.*"pil2-compiler"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$PROOFMAN_DIR/setup/pil2-stark/package.json" | head -n1)"
    [ -n "$pil2_compiler_version" ] || \
      { echo "could not read \"pil2-compiler\" from $PROOFMAN_DIR/setup/pil2-stark/package.json" >&2; exit 1; }
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