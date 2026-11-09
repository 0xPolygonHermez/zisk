#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .cache
builder=.cache/build-tools/bin/cargo-build-sbf
if [[ ! -x "$builder" ]]; then
    echo 'Install the pinned builder: cargo install cargo-build-sbf --version 4.6.0 --locked --root .cache/build-tools' >&2
    exit 1
fi
version=$("$builder" --version)
[[ "$version" == "cargo-build-sbf 4.6.0"* ]] || { echo 'Unexpected SBF builder version' >&2; exit 1; }
for crate in program consumer test-noop; do
    set --
    if [[ "$crate" == program && "${PROFILE_CUS:-0}" == 1 ]]; then set -- --features profile-cus; fi
    "$builder" --tools-version v1.57 --arch v2 --manifest-path "$crate/Cargo.toml" "$@" -- --locked 2>&1 | tee ".cache/build-$crate.log"
    # LLVM can report an invalid SBF stack frame without making the build command fail.
    if grep -Eq '^Error:|Branch target out of insn range' ".cache/build-$crate.log"; then exit 1; fi
done
