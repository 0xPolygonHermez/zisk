#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
bash tools/prepare-fixtures.sh
git diff --exit-code -- fixtures
cargo test --locked --manifest-path tools/Cargo.toml
cargo run --locked --manifest-path tools/Cargo.toml -- generate-profile --check
cargo fmt --manifest-path tools/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/Cargo.toml --all-targets -- -D warnings
cargo fmt --all -- --check
cargo fmt --manifest-path runtime-tests/Cargo.toml -- --check
cargo clippy --locked --workspace --all-targets --features trace -- -D warnings
cargo test --locked -p zisk-solana-verifier --features trace
node --test client/*.test.mjs
bash scripts/build-sbf.sh
cargo test --locked --manifest-path runtime-tests/Cargo.toml -- --nocapture 2>&1 | tee .cache/runtime.log
cargo run --locked --manifest-path tools/Cargo.toml -- check-benchmark .cache/runtime.log
