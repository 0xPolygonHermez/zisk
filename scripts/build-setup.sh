#!/usr/bin/env bash
#
# Dev orchestrator: drive compile-pil, setup, setup-snark, and stats from a
# single entry point. Reads https://storage.googleapis.com/zisk-setup/
#  over plain HTTPS for the cache lookup only
# Uploads are done by package-proving-key.sh.
#
# Modes (mutually exclusive)
# --------------------------
#   (default)          run setup --recursive. Cache lookup. On cache miss,
#                      writes <build-dir>/.input-hash so a follow-up
#                      package-proving-key.sh run can publish the sidecar.
#   --no-aggregation   run setup without -r. Bypasses the bucket entirely.
#   --snark            run setup-snark on top of an existing provingKey/.
#                      Uses local build/provingKey if present; otherwise
#                      requires a cache hit.
#   --compile-pil      run only frops + compile-pil + regenerate
#                      pil/src/pil_helpers/traces.rs. No setup, no bucket.
#   --stats            run frops + compile-pil + proofman-setup stats.
#                      No bucket interaction.
#
# pil-helpers (pil/src/pil_helpers/traces.rs) is regenerated as the last step
# of compile-pil in every mode that compiles the PIL, so traces.rs stays in
# sync with the freshly built pil/zisk.pilout. --skip-compile-pil skips both
# steps together (the on-disk traces.rs is assumed to match the reused pilout).
#
# Cache key
# ---------
# An input-side sha256 over:
#   - every *.pil under  pil/ state-machines/ precompiles/
#   - every *.pil under  ${PROOFMAN_DIR}/pil2-components/lib/std/pil
#   - state-machines/starkstructs.json
#   - the three *_fixed.bin files written by the frops generators
#   - pil2-compiler dep ref from ${PROOFMAN_DIR}/package.json
#   - pil2-stark-setup source string from `cargo metadata`
#
# Hash sidecar lives at https://storage.googleapis.com/zisk-setup/zisk-provingkey-<VERSION>.input-hash.
# It is uploaded by package-proving-key.sh, which reads <build-dir>/.input-hash
# written here after a successful cache-miss build.
#
# Env vars
#   PROOFMAN_DIR    override path to a pil2-proofman checkout. By default the
#                   checkout cargo fetched for the git dep in Cargo.toml is used
#                   (~/.cargo/git/checkouts/pil2-proofman-*/<rev>/).
#   OUT_DIR         where to extract on cache hit (default: $HOME/.zisk)
#
# https://storage.googleapis.com/zisk-setup/ is public-read — fetched over HTTPS
# Uploads (publishing) are done from package-proving-key.sh, which does require auth.

set -euo pipefail

usage() {
  cat <<EOF >&2
usage: $0 [--build-dir DIR] [--recursive-jobs N] [--setup-jobs N]
         [--skip-compile-pil] [--release] [-v|-vv|--verbose]
         [--compile-pil | --no-aggregation | --snark | --compressed-final | --stats]

  --build-dir DIR        Build directory. Default: build/. Used by setup as
                         output and by --snark / --compressed-final as input.
  --release              Look up the release-namespace bucket artifact
                         (zisk-provingkey-<VERSION>.*). Without this flag,
                         the "pre-<VERSION>" namespace is used, matching the
                         default upload namespace of package-proving-key.sh.
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
                         Faster local iteration. The <build-dir>/.input-hash
                         sidecar is NOT written (cache hash would not match
                         the reused pilout), so a follow-up
                         package-proving-key.sh will skip the sidecar upload.
  --compile-pil          Run only frops + compile-pil + pil-helpers regen
                         (writes pil/zisk.pilout and pil/src/pil_helpers/).
                         No setup, no bucket.
  --no-aggregation       Setup without -r. Bypasses the bucket entirely.
  --snark                Run setup-snark on top of an existing <build-dir>/provingKey/.
  --compressed-final     Re-run only vadcop_final_compressed on top of an
                         existing <build-dir>/provingKey/<name>/vadcop_final/.
                         No bucket interaction.
  --stats                Run proofman-setup stats. No bucket interaction.

To publish after a successful build, run:
  ./scripts/package-proving-key.sh --build-dir <build-dir>
EOF
  exit 1
}

MODE="build"   # build | no_aggregation | snark | compressed_final | stats | compile_pil
BUILD_DIR="build"
RECURSIVE_JOBS_ARG=""
SETUP_JOBS_ARG=""
SKIP_COMPILE_PIL=0
RELEASE=0
VERBOSE_COUNT=0

set_mode() {
  if [ "$MODE" != "build" ]; then
    echo "error: only one of --compile-pil, --no-aggregation, --snark, --compressed-final, --stats may be passed" >&2
    exit 1
  fi
  MODE="$1"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --build-dir)         BUILD_DIR="$2";          shift 2 ;;
    --recursive-jobs)    RECURSIVE_JOBS_ARG="$2"; shift 2 ;;
    --setup-jobs)        SETUP_JOBS_ARG="$2";     shift 2 ;;
    --skip-compile-pil)  SKIP_COMPILE_PIL=1;      shift ;;
    --release)           RELEASE=1;               shift ;;
    -v|--verbose)        VERBOSE_COUNT=$((VERBOSE_COUNT + 1)); shift ;;
    -vv)                 VERBOSE_COUNT=$((VERBOSE_COUNT + 2)); shift ;;
    --compile-pil)       set_mode compile_pil;       shift ;;
    --no-aggregation)    set_mode no_aggregation;    shift ;;
    --snark)             set_mode snark;             shift ;;
    --compressed-final)  set_mode compressed_final;  shift ;;
    --stats)             set_mode stats;             shift ;;
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

# Bucket is only relevant in build (cache lookup) and snark (cache check) modes.
# Reads are anonymous (zisk-setup is public-read), so no auth check needed —
# uploads happen in package-proving-key.sh, which has its own auth check.
USE_BUCKET=0
case "$MODE" in
  build) USE_BUCKET=1 ;;
  snark) [ -d "$BUILD_DIR/provingKey" ] || USE_BUCKET=1 ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${OUT_DIR:-$HOME/.zisk}"

command -v jq >/dev/null || { echo "jq not on PATH" >&2; exit 1; }

# Resolves PROOFMAN_DIR, defines generate_frops / compute_input_hash, and sets
# VERSION / BUCKET / PK_NAME / HASH_NAME / INCLUDE_PATHS. See lib/setup-common.sh
# for the contract.
. "$SCRIPT_DIR/lib/setup-common.sh"

# compile-pil shells into pil2-compiler from $PROOFMAN_DIR/node_modules. The
# cargo-managed checkout starts empty; install on demand so the user doesn't have to.
if [ ! -d "$PROOFMAN_DIR/node_modules" ]; then
  command -v npm >/dev/null || { echo "npm not on PATH (needed to install pil2-compiler in $PROOFMAN_DIR)" >&2; exit 1; }
  echo "==> npm install in $PROOFMAN_DIR (one-time)"
  (cd "$PROOFMAN_DIR" && npm install)
fi

# compile-pil resolves pil2com via PIL2C_EXEC, then ./node_modules/.bin (cwd), then
# walks up from the binary, then PATH. None of those reach the cargo git checkout,
# so point it at the binary explicitly.
PIL2C_EXEC_PATH="$PROOFMAN_DIR/node_modules/.bin/pil2com"
[ -x "$PIL2C_EXEC_PATH" ] || [ -L "$PIL2C_EXEC_PATH" ] \
  || { echo "pil2com missing at $PIL2C_EXEC_PATH after npm install" >&2; exit 1; }
export PIL2C_EXEC="$PIL2C_EXEC_PATH"

SNARKJS_PATH_DIR="$PROOFMAN_DIR/node_modules/snarkjs"
[ -d "$SNARKJS_PATH_DIR" ] \
  || { echo "snarkjs missing at $SNARKJS_PATH_DIR after npm install" >&2; exit 1; }
export SNARKJS_PATH="$SNARKJS_PATH_DIR"

if [ $USE_BUCKET -eq 1 ]; then
  command -v curl >/dev/null || { echo "curl not on PATH" >&2; exit 1; }
fi

echo "version: $VERSION  mode: $MODE"

run_compile_pil() {
  if [ $SKIP_COMPILE_PIL -eq 1 ]; then
    [ -f pil/zisk.pilout ] || { echo "--skip-compile-pil set but pil/zisk.pilout is missing" >&2; exit 1; }
    echo "==> compile-pil (SKIPPED — reusing pil/zisk.pilout and existing pil_helpers)"
    return
  fi
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
    "${VERBOSE_FLAGS[@]}"

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
      "${VERBOSE_FLAGS[@]}"
    echo "stats written to tmp/stats.txt"
    exit 0
    ;;

  snark)
    if [ -d "$BUILD_DIR/provingKey" ]; then
      echo "using existing $BUILD_DIR/provingKey/"
    else
      generate_frops
      LOCAL_HASH="$(compute_input_hash)"
      echo "local input hash: $LOCAL_HASH"

      remote_hash_tmp=$(mktemp)
      remote_hash=""
      if curl -fsSL "${BUCKET}/${HASH_NAME}" -o "$remote_hash_tmp" 2>/dev/null; then
        remote_hash="$(awk '{print $1}' "$remote_hash_tmp")"
      fi
      rm -f "$remote_hash_tmp"

      if [ -z "$remote_hash" ] || [ "$remote_hash" != "$LOCAL_HASH" ]; then
        echo "no recursive proving key cached for current inputs (remote=${remote_hash:-none}, local=$LOCAL_HASH)" >&2
        echo "rerun without --snark to build it, then ./scripts/package-proving-key.sh to share." >&2
        exit 1
      fi

      echo "cache hit — downloading ${PK_NAME} into $BUILD_DIR/"
      mkdir -p "$BUILD_DIR"
      tarball="$(mktemp --suffix=.tar.gz)"
      curl -fL --progress-bar "${BUCKET}/${PK_NAME}" -o "$tarball"
      rm -rf "$BUILD_DIR/provingKey"
      tar -xzf "$tarball" -C "$BUILD_DIR"
      rm -f "$tarball"
    fi

    PUBLICS_INFO="state-machines/publics.json"
    [ -f "$PUBLICS_INFO" ] || { echo "missing $PUBLICS_INFO — final.circom needs publics layout (nPublics, chunks, hasProgramVK)" >&2; exit 1; }

    PTAU_PATH="${PTAU_PATH:-../powersOfTau28_hez_final_27.ptau}"
    [ -f "$PTAU_PATH" ] || { echo "missing $PTAU_PATH — set PTAU_PATH=/path/to/ptau if elsewhere" >&2; exit 1; }

    echo "==> proofman-setup setup-snark"
    cargo run --release -p cargo-zisk -- proofman-setup setup-snark \
      --build-dir "$BUILD_DIR" \
      --publics-info "$PUBLICS_INFO" \
      --powers-of-tau "$PTAU_PATH"

    echo "done. to publish: ./scripts/package-proving-key.sh --build-dir $BUILD_DIR --snark"
    exit 0
    ;;

  compressed_final)
    [ -d "$BUILD_DIR/provingKey" ] || { echo "$BUILD_DIR/provingKey not found — run setup --recursive first" >&2; exit 1; }
    echo "==> proofman-setup setup-compressed-final"
    cargo run --release -p cargo-zisk -- proofman-setup setup-compressed-final --build-dir "$BUILD_DIR"
    echo "done. vadcop_final_compressed/ regenerated under $BUILD_DIR/provingKey/"
    echo "to republish the updated provingKey/: ./scripts/package-proving-key.sh --build-dir $BUILD_DIR"
    echo "(this re-uploads the full provingKey tarball, not just vadcop_final_compressed/)"
    exit 0
    ;;

  build|no_aggregation)
    generate_frops
    LOCAL_HASH="$(compute_input_hash)"
    echo "local input hash: $LOCAL_HASH"
    ;;

esac

# ----- build / no-aggregation ------------------------------------------------

if [ "$MODE" = "build" ]; then
  remote_hash_tmp=$(mktemp)
  remote_hash=""
  if curl -fsSL "${BUCKET}/${HASH_NAME}" -o "$remote_hash_tmp" 2>/dev/null; then
    remote_hash="$(awk '{print $1}' "$remote_hash_tmp")"
  fi
  rm -f "$remote_hash_tmp"

  if [ -n "$remote_hash" ] && [ "$remote_hash" = "$LOCAL_HASH" ]; then
    echo "cache hit — downloading ${PK_NAME}"
    mkdir -p "$OUT_DIR"
    tarball="$(mktemp --suffix=.tar.gz)"
    curl -fL --progress-bar "${BUCKET}/${PK_NAME}" -o "$tarball"
    rm -rf "$OUT_DIR/provingKey"
    tar -xzf "$tarball" -C "$OUT_DIR"
    rm -f "$tarball"
    echo "extracted to $OUT_DIR/provingKey — done"
    exit 0
  fi

  if [ -n "$remote_hash" ]; then
    echo "cache miss — remote hash ${remote_hash} != local ${LOCAL_HASH}"
  else
    echo "cache miss — no remote hash sidecar found"
  fi
fi

run_compile_pil

if [ "$MODE" = "build" ]; then
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
  "${setup_recursive_flag[@]}" \
  "${setup_jobs_flags[@]}"

# Drop the hash sidecar for package-proving-key.sh to pick up — but only when we
# really did a fresh build mode run from a current pilout. --skip-compile-pil
# reuses pil/zisk.pilout from a prior run, so the hash we computed wouldn't
# necessarily match the artifacts; better to leave no sidecar than a wrong one.
if [ "$MODE" = "build" ] && [ $SKIP_COMPILE_PIL -eq 0 ]; then
  echo "$LOCAL_HASH" > "$BUILD_DIR/.input-hash"
  echo "wrote $BUILD_DIR/.input-hash ($LOCAL_HASH)"
fi

if [ "$MODE" = "build" ]; then
  echo "done. to publish: ./scripts/package-proving-key.sh --build-dir $BUILD_DIR"
else
  echo "done."
fi
