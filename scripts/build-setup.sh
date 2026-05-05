#!/usr/bin/env bash
#
# Dev orchestrator: drive compile-pil, setup, setup-snark, and stats from a
# single entry point. Talks to gs://zisk-setup/ for the cache lookup only.
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
#   --stats            run frops + compile-pil + proofman-setup stats.
#                      No bucket interaction.
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
# Hash sidecar lives at  gs://zisk-setup/zisk-provingkey-<VERSION>.input-hash.
# It is uploaded by package-proving-key.sh, which reads <build-dir>/.input-hash
# written here after a successful cache-miss build.
#
# Env vars
#   PROOFMAN_DIR    override path to a pil2-proofman checkout. By default the
#                   checkout cargo fetched for the git dep in Cargo.toml is used
#                   (~/.cargo/git/checkouts/pil2-proofman-*/<rev>/).
#   OUT_DIR         where to extract on cache hit (default: $HOME/.zisk)
#
# Auth (when bucket is used): `gcloud auth login` or GOOGLE_APPLICATION_CREDENTIALS.

set -euo pipefail

usage() {
  cat <<EOF >&2
usage: $0 [--build-dir DIR] [--recursive-jobs N] [--setup-jobs N]
         [--skip-compile-pil] [--no-aggregation | --snark | --compressed-final | --stats]

  --build-dir DIR        Build directory. Default: build/. Used by setup as
                         output and by --snark / --compressed-final as input.
  --recursive-jobs N     Concurrent recursive1 air pipelines (circom + pil2com).
                         Default 1. Each job can use several GB; size by RAM.
                         Also settable via RECURSIVE_JOBS env var.
  --setup-jobs N         Concurrent non-recursive AIR setups (pil_info + I/O).
                         Default 1. Cheaper per job than --recursive-jobs.
                         Also settable via SETUP_JOBS env var.
  --skip-compile-pil     Reuse the existing pil/zisk.pilout instead of
                         recompiling. Faster local iteration. The
                         <build-dir>/.input-hash sidecar is NOT written
                         (cache hash would not match the reused pilout),
                         so a follow-up package-proving-key.sh will skip
                         the sidecar upload.
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

MODE="build"   # build | no_aggregation | snark | compressed_final | stats
BUILD_DIR="build"
RECURSIVE_JOBS_ARG=""
SETUP_JOBS_ARG=""
SKIP_COMPILE_PIL=0

set_mode() {
  if [ "$MODE" != "build" ]; then
    echo "error: only one of --no-aggregation, --snark, --compressed-final, --stats may be passed" >&2
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
    --no-aggregation)    set_mode no_aggregation;    shift ;;
    --snark)             set_mode snark;             shift ;;
    --compressed-final)  set_mode compressed_final;  shift ;;
    --stats)             set_mode stats;             shift ;;
    -h|--help)         usage ;;
    *) echo "unknown arg: $1" >&2; usage ;;
  esac
done

# Bucket is only relevant in build (cache+publish) and snark (cache check) modes.
# For --snark with an existing local provingKey/, we won't touch the bucket at all,
# so skip the upfront gsutil ls that triggers gcloud reauth prompts unnecessarily.
USE_BUCKET=0
case "$MODE" in
  build) USE_BUCKET=1 ;;
  snark) [ -d "$BUILD_DIR/provingKey" ] || USE_BUCKET=1 ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${OUT_DIR:-$HOME/.zisk}"
BUCKET="gs://zisk-setup"

command -v jq >/dev/null || { echo "jq not on PATH" >&2; exit 1; }

# Resolve the pil2-proofman checkout. PROOFMAN_DIR env wins (handy for local dev
# against an unpushed branch). Otherwise read it from cargo's git checkout for the
# `proofman` dep in Cargo.toml — that's the source that will actually be compiled
# into cargo-zisk, so it's also the one whose pil/package.json must feed the cache key.
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
  command -v gsutil >/dev/null || { echo "gsutil not on PATH" >&2; exit 1; }
  # Fail fast on missing GCS auth — better than running cargo for half an hour first.
  gsutil ls "${BUCKET}/" >/dev/null
fi

VERSION="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$ROOT_DIR/Cargo.toml")"
echo "version: $VERSION  mode: $MODE"

INCLUDE_PATHS="pil,${PROOFMAN_DIR}/pil2-components/lib/std/pil,state-machines,precompiles"
PK_NAME="zisk-provingkey-${VERSION}.tar.gz"
HASH_NAME="zisk-provingkey-${VERSION}.input-hash"

# ----- frops fixed data ------------------------------------------------------
# Required inputs to compile-pil and to the input-hash. Cheap to regenerate.
generate_frops() {
  echo "==> generating frops fixed data"
  cargo run --release --bin arith_frops_fixed_gen
  cargo run --release --bin binary_basic_frops_fixed_gen
  cargo run --release --bin binary_extension_frops_fixed_gen
}

compute_input_hash() {
  local pil_list
  pil_list=$(mktemp)
  find pil state-machines precompiles -type f -name '*.pil' >> "$pil_list"
  find "$PROOFMAN_DIR/pil2-components/lib/std/pil" -type f -name '*.pil' >> "$pil_list"
  sort -o "$pil_list" "$pil_list"

  local fixed_bins=(
    state-machines/arith/src/arith_frops_fixed.bin
    state-machines/binary/src/binary_basic_frops_fixed.bin
    state-machines/binary/src/binary_extension_frops_fixed.bin
  )
  local f
  for f in "${fixed_bins[@]}"; do
    [ -f "$f" ] || { echo "missing fixed binary: $f — run its generator first" >&2; rm -f "$pil_list"; return 1; }
  done

  local pil2_compiler_version pil2_stark_setup_source
  pil2_compiler_version="$(jq -r '.dependencies."pil2-compiler"' "$PROOFMAN_DIR/package.json")"
  [ -n "$pil2_compiler_version" ] && [ "$pil2_compiler_version" != "null" ] || \
    { echo "could not read .dependencies.pil2-compiler from $PROOFMAN_DIR/package.json" >&2; rm -f "$pil_list"; return 1; }
  pil2_stark_setup_source="$(cargo metadata --format-version 1 --no-deps \
    | jq -r '.packages[]|select(.name=="pil2-stark-setup")|.source // .manifest_path')"

  echo "hashing $(wc -l < "$pil_list") .pil files + starkstructs.json + ${#fixed_bins[@]} *_fixed.bin + tool refs" >&2
  {
    xargs -a "$pil_list" cat
    cat state-machines/starkstructs.json
    cat "${fixed_bins[@]}"
    printf 'pil2-compiler:%s\n' "$pil2_compiler_version"
    printf 'pil2-stark-setup:%s\n' "$pil2_stark_setup_source"
  } | sha256sum | awk '{print $1}'
  rm -f "$pil_list"
}

run_compile_pil() {
  if [ $SKIP_COMPILE_PIL -eq 1 ]; then
    [ -f pil/zisk.pilout ] || { echo "--skip-compile-pil set but pil/zisk.pilout is missing" >&2; exit 1; }
    echo "==> compile-pil (SKIPPED — reusing pil/zisk.pilout)"
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
    --no-proto-fixed-data
}

# ----- mode dispatch ---------------------------------------------------------

case "$MODE" in

  stats)
    generate_frops
    run_compile_pil
    echo "==> proofman-setup stats"
    cargo run --release -p cargo-zisk -- proofman-setup stats \
      --airout pil/zisk.pilout \
      -o tmp/stats.txt
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
      if gsutil cp "${BUCKET}/${HASH_NAME}" "$remote_hash_tmp" 2>/dev/null; then
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
      gsutil cp "${BUCKET}/${PK_NAME}" "$tarball"
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
  if gsutil cp "${BUCKET}/${HASH_NAME}" "$remote_hash_tmp" 2>/dev/null; then
    remote_hash="$(awk '{print $1}' "$remote_hash_tmp")"
  fi
  rm -f "$remote_hash_tmp"

  if [ -n "$remote_hash" ] && [ "$remote_hash" = "$LOCAL_HASH" ]; then
    echo "cache hit — downloading ${PK_NAME}"
    mkdir -p "$OUT_DIR"
    tarball="$(mktemp --suffix=.tar.gz)"
    gsutil cp "${BUCKET}/${PK_NAME}" "$tarball"
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

echo "done. to publish: ./scripts/package-proving-key.sh --build-dir $BUILD_DIR"
