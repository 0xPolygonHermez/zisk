#!/usr/bin/env bash
#
# Dev orchestrator: drive compile-pil, setup, setup-snark, and stats from a
# single entry point. Builds locally only — no bucket / network access. An
# optional local artifact cache (--cache-dir) lets a repeat build reuse a
# previously built provingKey/ keyed by the input hash, instead of rebuilding.
# To package the result (provingKey/circom/snark tarballs), use
# tools/test-env/package_setup.sh.
#
# Modes (mutually exclusive)
# --------------------------
#   (default)          run setup --recursive.
#   --no-aggregation   run setup without -r.
#   --snark            run setup-snark on top of an existing
#                      <build-dir>/provingKey/. Errors out if that directory
#                      is missing — populate it first with build-setup.sh
#                      (no flag).
#   --compile-pil      run only frops + compile-pil + regenerate
#                      pil/src/pil_helpers/traces.rs. No setup.
#   --compressed-final re-run only vadcop_final_compressed on top of an existing
#                      <build-dir>/provingKey/<name>/vadcop_final/.
#   --stats            run frops + compile-pil + proofman-setup stats.
#
# pil-helpers (pil/src/pil_helpers/traces.rs) is regenerated as the last step
# of compile-pil in every mode that compiles the PIL, so traces.rs stays in
# sync with the freshly built pil/zisk.pilout. --skip-compile-pil skips both
# steps together (the on-disk traces.rs is assumed to match the reused pilout).
#
# Input hash (the --cache-dir cache key)
# --------------------------------------
# An input-side sha256 over:
#   - every *.pil under  pil/ state-machines/ precompiles/
#   - every *.pil under  ${PROOFMAN_DIR}/pil2-components/lib/std/pil
#   - state-machines/starkstructs.json
#   - the three *_fixed.bin files written by the frops generators
#   - pil2-compiler dep ref from ${PROOFMAN_DIR}/package.json
#   - pil2-stark-setup source string from Cargo.lock
#
# Env vars
#   PROOFMAN_DIR    override path to a pil2-proofman checkout. By default the
#                   checkout cargo fetched for the git dep in Cargo.toml is used
#                   (~/.cargo/git/checkouts/pil2-proofman-*/<rev>/).

set -euo pipefail

usage() {
  cat <<EOF >&2
usage: $0 [--build-dir DIR] [--cache-dir DIR] [--recursive-jobs N] [--setup-jobs N]
         [--skip-compile-pil] [-v|-vv|--verbose]
         [--compile-pil | --no-aggregation | --snark | --compressed-final | --stats | --print-hash]

  --build-dir DIR        Build directory. Default: build/. Used by setup as
                         output and by --snark / --compressed-final as input.
  --cache-dir DIR        Local artifact cache root (default/--no-aggregation
                         only). On a cache hit the matching provingKey/ is
                         copied into <build-dir> and compile-pil + setup are
                         skipped; on a miss the fresh build is copied back in.
                         The cache key is PLATFORM/<input-hash>, so changing any
                         hashed input (see below) misses the cache. No bucket /
                         network access — this is a plain filesystem cache.
  --recursive-jobs N     Concurrent recursive1 air pipelines (circom + pil2com).
                         Default 1. Each job can use several GB; size by RAM.
                         Also settable via RECURSIVE_JOBS env var.
  --setup-jobs N         Concurrent non-recursive AIR setups (pil_info + I/O).
                         Default 1. Cheaper per job than --recursive-jobs.
                         Also settable via SETUP_JOBS env var.
  -v, --verbose          Verbose output. Repeat (-vv) or pass twice for
                         maximum verbosity. Forwarded to compile-pil and stats.
  --skip-compile-pil     Reuse the existing pil/zisk.pilout instead of
                         recompiling. Also skips the pil-helpers regen step.
                         Faster local iteration. With --cache-dir, the cache is
                         NOT populated (the reused pilout may not match the
                         computed input hash).
  --compile-pil          Run only frops + compile-pil + pil-helpers regen
                         (writes pil/zisk.pilout and pil/src/pil_helpers/).
                         No setup.
  --no-aggregation       Setup without -r.
  --snark                Run setup-snark on top of an existing
                         <build-dir>/provingKey/. Errors out if missing —
                         build it locally (./tools/setup/build-setup.sh) first.
  --compressed-final     Re-run only vadcop_final_compressed on top of an
                         existing <build-dir>/provingKey/<name>/vadcop_final/.
  --stats                Run proofman-setup stats.
  --print-hash           Print the build-input sha256 (the cache key) and exit.
                         Runs frops generation but no compile-pil / setup.

To package the result (provingKey + circom + snark tarballs), run:
  (cd tools/test-env && ./package_setup.sh)
EOF
  exit 1
}

MODE="build"   # build | no_aggregation | snark | compressed_final | stats | compile_pil | print_hash
BUILD_DIR="build"
CACHE_DIR=""
CACHE_HIT=0
CACHE_ENTRY=""
# Env defaults; the --recursive-jobs / --setup-jobs CLI flags override these below.
RECURSIVE_JOBS_ARG="${RECURSIVE_JOBS:-}"
SETUP_JOBS_ARG="${SETUP_JOBS:-}"
SKIP_COMPILE_PIL=0
VERBOSE_COUNT=0

set_mode() {
  if [ "$MODE" != "build" ]; then
    echo "error: only one of --compile-pil, --no-aggregation, --snark, --compressed-final, --stats, --print-hash may be passed" >&2
    exit 1
  fi
  MODE="$1"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --build-dir)         BUILD_DIR="$2";          shift 2 ;;
    --cache-dir)         CACHE_DIR="$2";          shift 2 ;;
    --recursive-jobs)    RECURSIVE_JOBS_ARG="$2"; shift 2 ;;
    --setup-jobs)        SETUP_JOBS_ARG="$2";     shift 2 ;;
    --skip-compile-pil)  SKIP_COMPILE_PIL=1;      shift ;;
    -v|--verbose)        VERBOSE_COUNT=$((VERBOSE_COUNT + 1)); shift ;;
    -vv)                 VERBOSE_COUNT=$((VERBOSE_COUNT + 2)); shift ;;
    --compile-pil)       set_mode compile_pil;       shift ;;
    --no-aggregation)    set_mode no_aggregation;    shift ;;
    --snark)             set_mode snark;             shift ;;
    --compressed-final)  set_mode compressed_final;  shift ;;
    --stats)             set_mode stats;             shift ;;
    --print-hash)        set_mode print_hash;        shift ;;
    -h|--help)         usage ;;
    *) echo "unknown arg: $1" >&2; usage ;;
  esac
done

if [ "$MODE" = "compile_pil" ] && [ $SKIP_COMPILE_PIL -eq 1 ]; then
  echo "error: --compile-pil and --skip-compile-pil are contradictory" >&2
  exit 1
fi

VERBOSE_FLAGS=()
for ((i = 0; i < VERBOSE_COUNT; i++)); do
  VERBOSE_FLAGS+=(-v)
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Repo root. This script no longer lives inside the repo tree (it ships
# standalone in the test-env image at /home/ziskuser/scripts), so it cannot be
# derived from SCRIPT_DIR. Prefer ZISK_REPO_DIR (exported by build_setup.sh and
# set in GHA), then the current directory (callers cd into the repo root before
# invoking us), and only then fall back to two levels up for legacy in-tree use.
if [[ -n "${ZISK_REPO_DIR:-}" ]]; then
  ROOT_DIR="$ZISK_REPO_DIR"
elif [[ -f "$PWD/Cargo.toml" ]]; then
  ROOT_DIR="$PWD"
else
  ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
fi
cd "$ROOT_DIR"

# Resolves PROOFMAN_DIR, defines generate_frops / compute_input_hash, and sets
# VERSION / INCLUDE_PATHS. See setup_common.sh for the contract.
. "$SCRIPT_DIR/setup_common.sh"

# ----- zisk-driven pil2-compiler override ------------------------------------
PROOFMAN_PKG="$PROOFMAN_DIR/package.json"
ZISK_PKG="$ROOT_DIR/package.json"
PROOFMAN_PKG_BAK=""

restore_proofman_pkg() {
  if [ -n "$PROOFMAN_PKG_BAK" ] && [ -f "$PROOFMAN_PKG_BAK" ]; then
    mv -f "$PROOFMAN_PKG_BAK" "$PROOFMAN_PKG"
    PROOFMAN_PKG_BAK=""
  fi
}

apply_zisk_compiler_override() {
  [ -f "$ZISK_PKG" ] || return 0
  local override
  override="$(sed -nE 's/.*"pil2-compiler"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$ZISK_PKG" | head -n1)"
  [ -n "$override" ] || return 0

  local current
  current="$(sed -nE 's/.*"pil2-compiler"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$PROOFMAN_PKG" | head -n1)"
  if [ "$override" = "$current" ]; then
    echo "==> zisk pins pil2-compiler to $override (matches proofman; no override needed)" >&2
    return 0
  fi

  echo "==> overriding pil2-compiler in proofman: $current -> $override (from $ZISK_PKG)" >&2
  PROOFMAN_PKG_BAK="$(mktemp)"
  cp "$PROOFMAN_PKG" "$PROOFMAN_PKG_BAK"
  trap restore_proofman_pkg EXIT
  local esc
  esc="$(printf '%s' "$override" | sed -e 's/[&/\]/\\&/g')"
  sed -i.tmp -E "s|(\"pil2-compiler\"[[:space:]]*:[[:space:]]*\")[^\"]+(\")|\1$esc\2|" "$PROOFMAN_PKG"
  rm -f "$PROOFMAN_PKG.tmp"
}

apply_zisk_compiler_override

# node/npm are only needed to run compile-pil (pil2com) and setup-snark (snarkjs),
# both from $PROOFMAN_DIR/node_modules. Set them up lazily — a --cache-dir hit or
# --skip-compile-pil never compiles or snarks, so it must not require node at all.
NODE_DEPS_READY=0
ensure_node_deps() {
  [ "$NODE_DEPS_READY" -eq 1 ] && return 0
  # compile-pil resolves pil2com via PIL2C_EXEC, then ./node_modules/.bin (cwd), then
  # walks up from the binary, then PATH. None of those reach the cargo git checkout,
  # so point it at the binary explicitly.
  local pil2c="$PROOFMAN_DIR/node_modules/.bin/pil2com"
  local snarkjs="$PROOFMAN_DIR/node_modules/snarkjs"

  # Reinstall from a clean tree so a partial or stale node_modules can't yield a
  # missing pil2com/snarkjs. The NODE_DEPS_READY guard caps this at one run per invocation.
  command -v npm >/dev/null || { echo "npm not on PATH (needed to install pil2-compiler in $PROOFMAN_DIR)" >&2; exit 1; }
  rm -rf "$PROOFMAN_DIR/node_modules"
  echo "==> npm install in $PROOFMAN_DIR"
  (cd "$PROOFMAN_DIR" && npm install)

  [ -x "$pil2c" ] || [ -L "$pil2c" ] \
    || { echo "pil2com missing at $pil2c after npm install" >&2; exit 1; }
  export PIL2C_EXEC="$pil2c"

  [ -d "$snarkjs" ] \
    || { echo "snarkjs missing at $snarkjs after npm install" >&2; exit 1; }
  export SNARKJS_PATH="$snarkjs"

  NODE_DEPS_READY=1
}

echo "version: $VERSION  mode: $MODE" >&2

run_compile_pil() {
  if [ $SKIP_COMPILE_PIL -eq 1 ]; then
    [ -f pil/zisk.pilout ] || { echo "--skip-compile-pil set but pil/zisk.pilout is missing" >&2; exit 1; }
    echo "==> compile-pil (SKIPPED — reusing pil/zisk.pilout and existing pil_helpers)"
    return
  fi
  ensure_node_deps
  echo "==> compile-pil"
  # --no-proto-fixed-data keeps fixed-column values out of the pilout protobuf
  # (they live on disk under tmp/fixed/ via --fixed-dir + --fixed-to-file). Avoids
  # the ~115 GB V8 heap peak on Keccakf-scale PILs.
  cargo run --release -p cargo-zisk -- proofman-setup compile-pil \
    --pil pil/zisk.pil \
    --include "$INCLUDE_PATHS" \
    --output pil/zisk.pilout \
    --fixed-dir tmp/fixed \
    --fixed-to-file \
    --no-proto-fixed-data \
    ${VERBOSE_FLAGS[@]+"${VERBOSE_FLAGS[@]}"}

  run_pil_helpers
}

# Regenerate pil/src/pil_helpers/{mod.rs,traces.rs} from the freshly compiled
# pilout. Invokes proofman-cli (from $PROOFMAN_DIR) — it's not in the zisk
# workspace, so we point cargo at its manifest explicitly. The -o flag
# overwrites the existing pil_helpers/ directory (it's always present in tree).
run_pil_helpers() {
  echo "==> pil-helpers (regenerating pil/src/pil_helpers/traces.rs)"
  cargo run --release --manifest-path "$PROOFMAN_DIR/Cargo.toml" -p proofman-cli -- \
    pil-helpers \
      --pilout pil/zisk.pilout \
      --path pil/src \
      -o
}

# ----- mode dispatch ---------------------------------------------------------

case "$MODE" in

  compile_pil)
    generate_frops
    run_compile_pil
    echo "done. pil/zisk.pilout and pil/src/pil_helpers/ regenerated."
    exit 0
    ;;

  stats)
    generate_frops
    run_compile_pil
    echo "==> proofman-setup stats"
    cargo run --release -p cargo-zisk -- proofman-setup stats \
      --airout pil/zisk.pilout \
      --starkstructs state-machines/starkstructs.json \
      -o tmp/stats.txt \
      ${VERBOSE_FLAGS[@]+"${VERBOSE_FLAGS[@]}"}
    echo "stats written to tmp/stats.txt"
    exit 0
    ;;

  snark)
    if [ ! -d "$BUILD_DIR/provingKey" ]; then
      echo "missing $BUILD_DIR/provingKey/ — populate it first:" >&2
      echo "  ./tools/setup/build-setup.sh --build-dir $BUILD_DIR    # build locally" >&2
      exit 1
    fi
    echo "using existing $BUILD_DIR/provingKey/"

    PUBLICS_INFO="state-machines/publics.json"
    [ -f "$PUBLICS_INFO" ] || { echo "missing $PUBLICS_INFO — final.circom needs publics layout (nPublics, chunks, hasProgramVK)" >&2; exit 1; }

    PTAU_PATH="${PTAU_PATH:-../powersOfTau28_hez_final_27.ptau}"
    [ -f "$PTAU_PATH" ] || { echo "missing $PTAU_PATH — set PTAU_PATH=/path/to/ptau if elsewhere" >&2; exit 1; }

    ensure_node_deps   # setup-snark needs snarkjs
    echo "==> proofman-setup setup-snark"
    cargo run --release -p cargo-zisk -- proofman-setup setup-snark \
      --build-dir "$BUILD_DIR" \
      --publics-info "$PUBLICS_INFO" \
      --powers-of-tau "$PTAU_PATH"

    echo "done. provingKeySnark/ under $BUILD_DIR/ — package with (cd tools/test-env && ./package_setup.sh)"
    exit 0
    ;;

  compressed_final)
    [ -d "$BUILD_DIR/provingKey" ] || { echo "$BUILD_DIR/provingKey not found — run setup --recursive first" >&2; exit 1; }
    ensure_node_deps
    echo "==> proofman-setup setup-compressed-final"
    cargo run --release -p cargo-zisk -- proofman-setup setup-compressed-final --build-dir "$BUILD_DIR"
    echo "done. vadcop_final_compressed/ regenerated under $BUILD_DIR/provingKey/"
    echo "to repackage the updated provingKey/: (cd tools/test-env && ./package_setup.sh)"
    exit 0
    ;;

  print_hash)
    # Keep stdout clean: only compute_input_hash's 64-hex line goes to stdout
    # (its own progress already goes to stderr). frops generation is noisy, so
    # send its output to stderr too — consumers capture stdout as the hash.
    generate_frops 1>&2
    compute_input_hash
    exit 0
    ;;

  build|no_aggregation)
    generate_frops
    LOCAL_HASH="$(compute_input_hash)"
    echo "local input hash: $LOCAL_HASH"

    # ----- local artifact cache lookup (opt-in via --cache-dir) --------------
    if [ -n "$CACHE_DIR" ]; then
      # PLATFORM mirrors tools/test-env's get_platform: ZISKUP_PLATFORM override,
      # else lowercased `uname -s`. The aggregation mode is part of the key so a
      # recursive build and a --no-aggregation build never collide (the input
      # hash itself does not encode -r).
      cache_platform="$(printf '%s' "${ZISKUP_PLATFORM:-$(uname -s)}" | tr '[:upper:]' '[:lower:]')"
      short_hash="${LOCAL_HASH:0:4}${LOCAL_HASH: -4}"
      cache_key="$short_hash"
      [ "$MODE" = "no_aggregation" ] && cache_key="${short_hash}-no-aggregation"
      CACHE_ENTRY="$CACHE_DIR/$cache_platform/$cache_key"

      if [ -d "$CACHE_ENTRY/provingKey" ]; then
        echo "==> cache hit: $CACHE_ENTRY (skipping compile-pil + setup)"
        rm -rf "$BUILD_DIR/provingKey"
        mkdir -p "$BUILD_DIR"
        cp -R "$CACHE_ENTRY/provingKey" "$BUILD_DIR/provingKey"
        CACHE_HIT=1
      else
        echo "==> cache miss: $CACHE_ENTRY (will build, then populate)"
      fi
    fi
    ;;

esac

# ----- build / no-aggregation ------------------------------------------------

if [ "$CACHE_HIT" -eq 0 ]; then
  run_compile_pil

  if [ "$MODE" = "build" ]; then
    ensure_node_deps
    echo "==> proofman-setup setup --recursive"
    setup_recursive_flag=(--recursive)
  else
    echo "==> proofman-setup setup (no aggregation)"
    setup_recursive_flag=()
  fi

  setup_jobs_flags=()
  [ -n "$RECURSIVE_JOBS_ARG" ] && setup_jobs_flags+=(--recursive-jobs "$RECURSIVE_JOBS_ARG")
  [ -n "$SETUP_JOBS_ARG" ]     && setup_jobs_flags+=(--setup-jobs "$SETUP_JOBS_ARG")

  rm -rf "$BUILD_DIR/provingKey"
  cargo run --release -p cargo-zisk -- proofman-setup setup \
    --airout pil/zisk.pilout \
    --build-dir "$BUILD_DIR" \
    --fixed-dir tmp/fixed \
    --stark-structs state-machines/starkstructs.json \
    ${setup_recursive_flag[@]+"${setup_recursive_flag[@]}"} \
    ${setup_jobs_flags[@]+"${setup_jobs_flags[@]}"}

  # Populate the local cache with the freshly built provingKey, keyed by the
  # input hash. --skip-compile-pil reuses a prior pilout that may not match the
  # computed hash, so don't poison the cache in that case.
  if [ -n "$CACHE_DIR" ] && [ $SKIP_COMPILE_PIL -eq 0 ]; then
    echo "==> caching provingKey to $CACHE_ENTRY"
    rm -rf "$CACHE_ENTRY"
    mkdir -p "$CACHE_ENTRY"
    cp -R "$BUILD_DIR/provingKey" "$CACHE_ENTRY/provingKey"
  fi
fi

if [ "$MODE" = "build" ]; then
  echo "done. to package: (cd tools/test-env && ./package_setup.sh)"
else
  echo "done."
fi
