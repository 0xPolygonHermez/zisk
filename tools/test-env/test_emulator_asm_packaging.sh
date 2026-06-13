#!/usr/bin/env bash
#
# Verifies the release tarball packages everything an installed worker needs to
# build emulator-asm at runtime. The worker (no cargo) runs `make` there,
# linking the prebuilt libziskc.a and compiling against lib-c/c/src headers; if
# release.yml stops staging either, ROM setup dies with `cannot find -lziskc`.
#
# It runs release.yml's real "Copy binaries" step (parsed from the workflow) and
# then the worker's link step against the result — so it guards the actual
# packaging, not a copy of it. Linux only.
#
# Usage: tools/test-env/test_emulator_asm_packaging.sh
# Env:   REPO (default: git toplevel)   ELF (default: emulator/benches/data/my.elf)
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "SKIP: Linux-only (lib-c's C build is Linux-only)."
  exit 0
fi

REPO="${REPO:-$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)}"
ELF="${ELF:-$REPO/emulator/benches/data/my.elf}"
RELEASE_YML="$REPO/.github/workflows/release.yml"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }
step() { echo "==> $*"; }

[[ -f "$ELF" ]]         || fail "test ELF not found: $ELF"
[[ -f "$RELEASE_YML" ]] || fail "release.yml not found at $RELEASE_YML"
command -v python3 >/dev/null || fail "python3 required to parse release.yml"
python3 -c 'import yaml' 2>/dev/null \
  || fail "Python PyYAML required to parse release.yml (pip install pyyaml / apt install python3-yaml)"

# Build the artifacts the staged step copies and the test exercises: ziskclib +
# lib-c (libziskclib.a / libziskc.a) and riscv2zisk (used to generate emu.asm).
step "Building libziskclib.a + libziskc.a + riscv2zisk"
cargo build --release -p ziskclib -p lib-c -p zisk-core --bin riscv2zisk --manifest-path "$REPO/Cargo.toml" \
  || fail "cargo build of ziskclib/lib-c/riscv2zisk failed"
[[ -f "$REPO/target/zisk-libs/libziskc.a" ]] \
  || fail "libziskc.a not at target/zisk-libs — lib-c build.rs did not publish it"

step "Running release.yml's 'Copy binaries' step (the staging under test)"
SCRIPT="$(python3 - "$RELEASE_YML" <<'PY'
import sys, yaml
wf = yaml.safe_load(open(sys.argv[1]))
for job in wf.get("jobs", {}).values():
    for stp in job.get("steps", []):
        if stp.get("name") == "Copy binaries":
            print(stp["run"]); sys.exit(0)
sys.exit("could not find 'Copy binaries' step in release.yml")
PY
)" || fail "failed to extract 'Copy binaries' step from release.yml"

# Run the step in a sandbox: stubs for the release binaries we don't build, real
# symlinks for the trees whose staging we verify. TARGET="" collapses the step's
# target/${TARGET}/release path onto the local target/release.
SANDBOX="$WORK/repo"
mkdir -p "$SANDBOX/target/release"
for b in cargo-zisk cargo-zisk-dev zisk-worker ziskemu zisk-coordinator riscv2zisk libziskclib.a; do
  cp "$REPO/target/release/$b" "$SANDBOX/target/release/" 2>/dev/null || : > "$SANDBOX/target/release/$b"
done
ln -s "$REPO/target/zisk-libs" "$SANDBOX/target/zisk-libs"
ln -s "$REPO/emulator-asm"     "$SANDBOX/emulator-asm"
ln -s "$REPO/lib-c"            "$SANDBOX/lib-c"
ln -s "$REPO/ziskup"           "$SANDBOX/ziskup"

( cd "$SANDBOX" && export TARGET="" PLATFORM_NAME="linux" ARCH="amd64" && eval "$SCRIPT" ) \
  || fail "release.yml 'Copy binaries' step failed to execute"
DIST="$SANDBOX/zisk-dist"
[[ -f "$DIST/zisk/target/zisk-libs/libziskc.a" ]] \
  || fail "release.yml did NOT stage libziskc.a into zisk/target/zisk-libs"

# Use the riscv2zisk the step STAGED (zisk-dist/bin), matching the installed
# worker and confirming release.yml ships a runnable one.
step "Generating emu.asm with staged riscv2zisk (--gen=1)"
RISCV2ZISK="$DIST/bin/riscv2zisk"
[[ -x "$RISCV2ZISK" ]] || fail "release.yml did NOT stage a runnable riscv2zisk into bin/"
"$RISCV2ZISK" "$ELF" "$DIST/zisk/emulator-asm/src/emu.asm" --gen=1 >/dev/null \
  || fail "staged riscv2zisk failed to generate emu.asm"

# The worker's exact step: `make` in the staged emulator-asm, no cargo. Links
# -lziskc only if the staging above placed libziskc.a where the Makefile's -L looks.
step "Building emulator-asm the worker's way (make, TRACE_TARGET=MT)"
EMUASM="$DIST/zisk/emulator-asm"
( cd "$EMUASM" && make clean >/dev/null 2>&1; \
  make EMU_PATH=src/emu.asm OUT_PATH=build/ziskemuasm-mt TRACE_TARGET=MT ) \
  > "$WORK/make.log" 2>&1 && RC=0 || RC=$?

if grep -q "cannot find -lziskc" "$WORK/make.log"; then
  tail -15 "$WORK/make.log"
  fail "REGRESSION: 'cannot find -lziskc' — release.yml no longer stages libziskc.a."
fi
[[ "$RC" -eq 0 ]] || { tail -25 "$WORK/make.log"; fail "make failed (rc=$RC) for a non-lib reason; see above."; }
[[ -f "$EMUASM/build/ziskemuasm-mt" ]] || fail "make succeeded but produced no binary"

step "PASS: a release-packaged worker can build emulator-asm (links libziskc.a)"
