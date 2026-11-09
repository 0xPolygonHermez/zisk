#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
seconds=${1:-60}
[[ "$seconds" =~ ^[0-9]+$ && "$seconds" -gt 0 ]] || { echo 'Duration must be positive seconds' >&2; exit 1; }
python3 tools/seed_fuzz.py
mkdir -p .cache
for target in verify buffer_state; do
    max_len=7297
    if [[ "$target" == buffer_state ]]; then max_len=384; fi
    RUSTUP_TOOLCHAIN=nightly-2026-02-12 .cache/fuzz-tools/bin/cargo-fuzz run "$target" -- \
        -max_total_time="$seconds" -max_len="$max_len" -seed=17012026 -print_final_stats=1 2>&1 | tee ".cache/fuzz-$target.log"
done
git diff --exit-code -- fuzz/Cargo.lock
