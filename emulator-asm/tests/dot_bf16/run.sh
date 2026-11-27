#!/bin/bash
# Cross-checks the assembly emulator's dot_bf16 C routine (src/dot_bf16/dot_bf16.c) against the
# Rust reference (zisk_definitions::dot_bf16): generates random cases with the reference (BF16,
# MXFP4 and NVFP4 weights; infinities, NaN, NaN scales, subnormals, cancellation, ties, overflow
# and random block sizes), then runs the C routine on them and compares every result.
#
#   emulator-asm/tests/dot_bf16/run.sh [CASES]      (default 450000)
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
SRC="$DIR/../../src/dot_bf16"
OUT="${TMPDIR:-/tmp}/dot-bf16-cases.$$"
mkdir -p "$OUT"
trap 'rm -rf "$OUT"' EXIT
cargo build --release --quiet --manifest-path "$DIR/Cargo.toml" --target-dir "$OUT/target"
"$OUT/target/release/dot-bf16-cases" "${1:-450000}" "$OUT/cases.bin"
gcc -O2 -I"$SRC" "$DIR/check.c" "$SRC/dot_bf16.c" -o "$OUT/check"
"$OUT/check" "$OUT/cases.bin"
