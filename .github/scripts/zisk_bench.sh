#!/usr/bin/env bash
#
# zisk_bench.sh — build ziskemu + a fixed set of guest ELFs from the *current*
# working tree, emulate each guest with `ziskemu -X`, and dump the per-program
# REPORT (STEPS + COST DISTRIBUTION) into <OUTDIR>/<program>.txt.
#
# It is run twice by the cycle-tracking workflow: once on the PR tree and once
# on the base-branch tree (selected via BENCH_REPO_DIR). Building from the
# selected tree each time means the diff reflects changes to the emulator / cost
# model *and* to the guest sources.
#
# Usage: zisk_bench.sh <OUTDIR>
#
# Requirements (provided by the workflow before calling this):
#   - system deps (tools/test-env/install_deps.sh)
#   - BENCH_REPO_DIR (optional): repo tree to build + benchmark; defaults to
#     GITHUB_WORKSPACE. The PR run uses the default; the base run points it at the
#     base-branch checkout, so this (PR) copy of the script drives both runs and
#     the methodology stays fixed while only the code under test varies.
#
set -euo pipefail

OUTDIR="${1:?usage: zisk_bench.sh <OUTDIR>}"
mkdir -p "$OUTDIR"

REPO="${BENCH_REPO_DIR:-${GITHUB_WORKSPACE:-$(git rev-parse --show-toplevel)}}"
cd "$REPO"

# If the expected guest ELF directory doesn't exist, skip the benchmarks.
if [[ ! -d "$REPO/test-artifacts/programs" ]]; then
  echo "WARNING: '$REPO/test-artifacts/programs' not found; skipping benchmarks (no reports produced)." >&2
  exit 0
fi

# The set of guest programs to benchmark. Each name must match both the crate
# `package.name` under test-artifacts/programs/<name> and the emitted ELF name.
PROGRAMS=(bigint bls12_381 bn254 diagnostic hashes secp256k1 secp256r1 uint256)

# Guest crates inherit this version from test-artifacts/programs/Cargo.toml
GUEST_VERSION="0.1.0"

echo "::group::Build host tools (cargo-zisk, ziskemu)"
cargo build --release -p cargo-zisk -p ziskemu --bin cargo-zisk --bin ziskemu
echo "::endgroup::"

CARGO_ZISK="$REPO/target/release/cargo-zisk"
ZISKEMU="$REPO/target/release/ziskemu"

echo "::group::Install ZisK rust toolchain"
"$CARGO_ZISK" toolchain install
echo "::endgroup::"

ELF_DIR="$REPO/test-artifacts/programs/target/elf/riscv64ima-zisk-zkvm-elf/release"

echo "::group::Build guest ELFs"
# Build each guest individually and tolerate failures: a guest that is new in
# this PR does not exist on the base branch, so its build fails on the base pass
# with "did not match any packages". Skipping (rather than aborting) lets the
# rest still benchmark; the diff renders the missing base side as N/A.
built=()
pushd "$REPO/test-artifacts/programs" >/dev/null
for prog in "${PROGRAMS[@]}"; do
  if "$CARGO_ZISK" build --release "--package=${prog}@${GUEST_VERSION}"; then
    built+=("$prog")
  else
    echo "WARNING: build failed for '$prog'; skipping" >&2
  fi
done
popd >/dev/null
echo "::endgroup::"

for prog in "${built[@]}"; do
  elf="$ELF_DIR/$prog"
  if [[ ! -f "$elf" ]]; then
    echo "WARNING: build reported success but no ELF for '$prog'; skipping" >&2
    continue
  fi
  echo "::group::Emulate $prog"
  # -X prints the REPORT/COST DISTRIBUTION summary to stdout.
  # No input file is needed: none of these guests read from stdin.
  "$ZISKEMU" -e "$elf" -X | tee "$OUTDIR/$prog.txt"
  echo "::endgroup::"
done

echo "Reports written to $OUTDIR:"
ls -1 "$OUTDIR"
