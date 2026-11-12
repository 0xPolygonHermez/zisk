#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .cache
package=proofman-pilfflonk-1.4.0-alpha
if [[ ! -f ".cache/$package.crate" ]]; then
    curl -fsSL --retry 2 -o ".cache/$package.crate" "https://static.crates.io/crates/proofman-pilfflonk/$package.crate"
fi
printf '%s  %s\n' f7c0a2aeb1ddd06d7b8aa4c786eea4cb4f82bdf7e02c1ca75259aa2570244cf4 ".cache/$package.crate" | shasum -a 256 -c -
tar -xzf ".cache/$package.crate" -C .cache
npm ci --ignore-scripts --no-audit --no-fund
cargo run --locked --manifest-path tools/extract-fixture/Cargo.toml
node tools/reference.mjs
node --test tools/fixture.test.mjs
forge test --root .
