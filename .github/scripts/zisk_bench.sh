#!/usr/bin/env bash
#
# zisk_bench.sh — build ziskemu + a fixed set of guest ELFs from the *current*
# working tree, emulate each guest with `ziskemu -X`, and dump the per-program
# REPORT (STEPS + COST DISTRIBUTION) into <OUTDIR>/<program>.txt.
#
# It is run twice by the cycle-tracking workflow: once on the PR tree and once
# on the base-branch tree. Building from the current tree each time means the
# diff reflects changes to the emulator / cost model *and* to the guest sources.
#
# Usage: zisk_bench.sh <OUTDIR>
#
# Requirements (installed by the workflow before calling this):
#   - system deps (tools/test-env/install_deps.sh)
#   - the ZisK rust toolchain (`cargo-zisk toolchain install`)
#
set -euo pipefail

OUTDIR="${1:?usage: zisk_bench.sh <OUTDIR>}"
mkdir -p "$OUTDIR"

REPO="${GITHUB_WORKSPACE:-$(git rev-parse --show-toplevel)}"
cd "$REPO"

# The set of guest programs to benchmark. Each name must match both the crate
# `package.name` under test-artifacts/programs/<name> and the emitted ELF name.
PROGRAMS=(diagnostic modexp secp256k1 uint256)

echo "::group::Build host tools (cargo-zisk, ziskemu)"
cargo build --release -p cargo-zisk -p ziskemu --bin cargo-zisk --bin ziskemu
echo "::endgroup::"

CARGO_ZISK="$REPO/target/release/cargo-zisk"
ZISKEMU="$REPO/target/release/ziskemu"

echo "::group::Build guest ELFs"
# cargo-zisk build drives `cargo +zisk build` in the current dir, emitting ELFs
# to target/elf/<ZISK_TARGET>/release/<name>. Run it from the nested guest
# workspace so it picks up the right members and profile flags.
(
  cd "$REPO/test-artifacts/programs"
  "$CARGO_ZISK" build --release "${PROGRAMS[@]/#/--package=}"
)
echo "::endgroup::"

ELF_DIR="$REPO/test-artifacts/programs/target/elf/riscv64ima-zisk-zkvm-elf/release"

for prog in "${PROGRAMS[@]}"; do
  elf="$ELF_DIR/$prog"
  if [[ ! -f "$elf" ]]; then
    echo "ERROR: expected ELF not found: $elf" >&2
    exit 1
  fi
  echo "::group::Emulate $prog"
  # -X prints the REPORT/COST DISTRIBUTION summary to stdout. No input file is
  # needed: none of these guests read from stdin. No proving key is required.
  "$ZISKEMU" -e "$elf" -X | tee "$OUTDIR/$prog.txt"
  echo "::endgroup::"
done

echo "Reports written to $OUTDIR:"
ls -1 "$OUTDIR"
