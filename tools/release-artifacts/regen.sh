#!/usr/bin/env bash
#
# Regenerate each family's release verification artifacts from a fresh setup: the verifiers
# (verifier/src/<f>/), keys.rs and the pilfflonk contracts (zisk-contracts/<f>/); with --prove,
# also the fib_mod fixtures. Each family's build is deleted once installed, unless kept.
#
# usage: tools/release-artifacts/regen.sh [options]
#   --families LIST       Comma-separated, of Poseidon1, Poseidon2, blake3. Default: all three.
#   --build-root DIR      Where each family is built, as DIR/<f>. Default: build-release.
#   --setup-jobs N        Forwarded to setup_build.sh. Default: 4.
#   --recursive-jobs N    Forwarded to setup_build.sh. Default: 4.
#   --ptau PATH           Powers of tau for setup-snark. Default: setup_common.sh's PTAU_PATH.
#   --skip-setup          Reuse DIR/<f> as it is (implies --keep-build).
#   --compressed-only     Only the compressed stage changed: no SNARK, keep the pilfflonk
#                         artifacts, and wrap the committed leaf fixture.
#   --keep-build          Keep every DIR/<f> after its artifacts are installed.
#   --keep LIST           Keep DIR/<f> of these families only (comma-separated).
#   --prove               Also prove the fixtures (on the GPU).
#   --skip-checks         Install only.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

FAMILIES="Poseidon1,Poseidon2,blake3"
BUILD_ROOT="build-release"
SETUP_JOBS=4
RECURSIVE_JOBS=4
PTAU_ENV=()
SKIP_SETUP=0
KEEP_BUILD=0
KEEP_FAMILIES=""
SKIP_CHECKS=0
PROVE=0
COMPRESSED_ONLY=0
# The families with a hardhat test of their pilfflonk verifier.
SOLIDITY_FAMILIES="blake3"

usage() { sed -n '/^# usage:/,/^$/p' "$0" | sed 's/^# \{0,1\}//'; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --families)       FAMILIES="$2";       shift 2 ;;
    --build-root)     BUILD_ROOT="$2";     shift 2 ;;
    --setup-jobs)     SETUP_JOBS="$2";     shift 2 ;;
    --recursive-jobs) RECURSIVE_JOBS="$2"; shift 2 ;;
    --ptau)           PTAU_ENV=("PTAU_PATH=$2"); shift 2 ;;
    --skip-setup)     SKIP_SETUP=1; KEEP_BUILD=1; shift ;;
    --keep-build)     KEEP_BUILD=1;        shift ;;
    --keep)           KEEP_FAMILIES="$2";  shift 2 ;;
    --skip-checks)    SKIP_CHECKS=1;       shift ;;
    --prove)          PROVE=1;             shift ;;
    --compressed-only) COMPRESSED_ONLY=1; shift ;;
    -h|--help)        usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

stem() {
  case "$1" in
    Poseidon1) echo poseidon1 ;;
    Poseidon2) echo poseidon2 ;;
    blake3)    echo blake3 ;;
    *) echo "unknown family: $1 (expected Poseidon1, Poseidon2 or blake3)" >&2; exit 2 ;;
  esac
}

IFS=',' read -r -a FAMILY_LIST <<< "$FAMILIES"
for family in "${FAMILY_LIST[@]}"; do stem "$family" > /dev/null; done

log() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

# The expression-kernel codegen writes GBs of temporary sources: keep them off a small /tmp.
mkdir -p "$BUILD_ROOT/tmp"
TMPDIR="$(cd "$BUILD_ROOT/tmp" && pwd)"
export TMPDIR

log "Building cargo-zisk-dev"
cargo build --release -p cargo-zisk --bin cargo-zisk-dev
ZISK_DEV="$REPO_ROOT/target/release/cargo-zisk-dev"

if [[ $PROVE -eq 1 ]]; then
  log "Building fib_mod"
  # test-artifacts is its own workspace; its build script builds the guests with the host profile.
  cargo build --release --manifest-path test-artifacts/Cargo.toml --no-default-features \
    --features fib_mod
  FIB_MOD_ELF="$REPO_ROOT/test-artifacts/programs/target/elf/riscv64ima-zisk-zkvm-elf/release/fib_mod"
  [[ -f "$FIB_MOD_ELF" ]] || { echo "fib_mod ELF not found at $FIB_MOD_ELF" >&2; exit 1; }
fi

# The PIL is the same for every family: compile it (and regenerate traces.rs) once.
COMPILE_PIL_ARG=()
MANIFESTS=()

for family in "${FAMILY_LIST[@]}"; do
  f="$(stem "$family")"
  build="$BUILD_ROOT/$f"

  have_build=0
  [[ -d "$build/provingKey" ]] && have_build=1
  if [[ $SKIP_SETUP -eq 1 && $have_build -eq 0 ]]; then
    echo "$build/provingKey does not exist: build $family without --skip-setup" >&2
    exit 1
  fi
  snark=1
  [[ $COMPRESSED_ONLY -eq 1 ]] && snark=0

  if [[ $COMPRESSED_ONLY -eq 1 && $have_build -eq 1 ]]; then
    log "[$family] vadcop_final_compressed setup in $build"
    env HASH_MODE="$family" tools/test-env/setup_build.sh --build-dir "$build" --compressed-final
  elif [[ $SKIP_SETUP -eq 0 ]]; then
    log "[$family] setup into $build"
    rm -rf "$build"
    tools/test-env/setup_build.sh --build-dir "$build" --hash-mode "$family" \
      --setup-jobs "$SETUP_JOBS" --recursive-jobs "$RECURSIVE_JOBS" "${COMPILE_PIL_ARG[@]}"
    COMPILE_PIL_ARG=(--skip-compile-pil)
  fi

  # Also when --skip-setup resumes a build whose SNARK setup did not finish.
  if [[ $snark -eq 1 && ( $SKIP_SETUP -eq 0 || ! -f "$build/provingKeySnark/final/ZiskVerifier.sol" ) ]]; then
    log "[$family] pilfflonk SNARK setup"
    env "${PTAU_ENV[@]}" FINAL_SNARK=pilfflonk HASH_MODE="$family" \
      tools/test-env/setup_build.sh --build-dir "$build" --snark
  fi

  log "[$family] install"
  install_args=()
  [[ $snark -eq 0 ]] && install_args=(--no-snark)
  "$ZISK_DEV" release-artifacts install --family "$family" --build-dir "$build" \
    --manifest-out "$BUILD_ROOT/$f.json" --repo . "${install_args[@]}"
  MANIFESTS+=("$BUILD_ROOT/$f.json")
  # After each family, so an interrupted run leaves keys.rs with every family it finished.
  "$ZISK_DEV" release-artifacts render --manifests "${MANIFESTS[@]}" --repo .

  if [[ $PROVE -eq 1 ]]; then
    log "[$family] GPU expression kernels"
    tools/test-env/setup_build.sh --build-dir "$build" --gen-exps-only

    proofs="$build/proofs"
    rm -rf "$proofs"
    mkdir -p "$proofs"
    # The pilfflonk wrap allocates ~0.7 GB on the device after the prover's buffers; the
    # default headroom (756 MB) leaves it ~0.6 GB on a 32 GB GPU.
    export PROOFMAN_GPU_HEADROOM_MB="${PROOFMAN_GPU_HEADROOM_MB:-1536}"
    snark_key=()
    [[ $snark -eq 1 ]] && snark_key=(-w "$build/provingKeySnark")
    if [[ $COMPRESSED_ONLY -eq 1 ]]; then
      # The leaf is unchanged (install checked its key): wrap the committed one.
      cp "verifier/tests/fixtures/${f}_leaf.bin" "$proofs/leaf.bin"
    else
      log "[$family] prove fib_mod: leaf"
      "$ZISK_DEV" prove -e "$FIB_MOD_ELF" -i "inline://[[100],[233]]" \
        -k "$build/provingKey" "${snark_key[@]}" --gpu -y -o "$proofs/leaf.bin"
    fi
    # The other stages wrap that leaf rather than re-run the guest.
    wrap() {
      "$ZISK_DEV" wrap-proof -p "$proofs/leaf.bin" -k "$build/provingKey" "${snark_key[@]}" \
        --gpu "$@"
    }
    log "[$family] wrap fib_mod: compressed"
    wrap -c -o "$proofs/compressed.bin"
    if [[ $snark -eq 1 && ",$SOLIDITY_FAMILIES," == *",$family,"* ]]; then
      log "[$family] wrap fib_mod: pilfflonk"
      wrap --plonk -o "$proofs/pilfflonk.bin"
    fi
    # A wrapped committed leaf commits to its own program, not to today's ELF's.
    program=(--elf "$FIB_MOD_ELF")
    [[ $COMPRESSED_ONLY -eq 1 ]] && program=(--program-vk "verifier/tests/fixtures/${f}_program_vk.json")
    "$ZISK_DEV" release-artifacts fixtures --family "$family" --proofs-dir "$proofs" \
      "${program[@]}" --repo .
  fi

  # The generator's verifiers are not rustfmt-formatted; the committed ones are. Per family, so
  # a later family's failure leaves the finished ones formatted.
  cargo fmt -p zisk-verifier

  if [[ $KEEP_BUILD -eq 0 && ",$KEEP_FAMILIES," != *",$family,"* ]]; then
    log "[$family] removing $build"
    rm -rf "$build"
  fi
done

if [[ $SKIP_CHECKS -eq 0 ]]; then
  log "Checks"
  cargo test -p cargo-zisk release_artifacts
  cargo test -p zisk-verifier -p zisk-common
  cargo check -p ziskos
  (cd zisk-contracts && npm install --no-audit --no-fund && npx hardhat compile)
  for f in ${SOLIDITY_FAMILIES//,/ }; do
    [[ -f "zisk-contracts/test/${f}_pilfflonk.bin" ]] || continue
    "$ZISK_DEV" export-solidity-calldata -p "zisk-contracts/test/${f}_pilfflonk.bin" \
      -o "zisk-contracts/fixtures/$f.json"
    (cd zisk-contracts && ZISK_SOLIDITY_FIXTURE="fixtures/$f.json" \
      ZISK_SOLIDITY_CONTRACT="$f/ZiskVerifier.sol:ZiskVerifier" npx hardhat test)
  done
fi

log "Done. Review with: git status -- verifier zisk-contracts"
