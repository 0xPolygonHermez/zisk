#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
bash tools/prepare-fixtures.sh
git diff --exit-code -- fixtures
python3 -m unittest discover -s tools -p 'test_*.py'
python3 tools/generate_profile.py --check
cargo fmt --all -- --check
cargo fmt --manifest-path runtime-tests/Cargo.toml -- --check
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
cargo clippy --locked --workspace --all-targets --features trace -- -D warnings
cargo test --locked -p zisk-solana-verifier --features trace
node --test client/*.test.mjs
bash scripts/build-sbf.sh
cargo test --locked --manifest-path runtime-tests/Cargo.toml -- --nocapture 2>&1 | tee .cache/runtime.log
python3 tools/check_benchmark.py .cache/runtime.log
