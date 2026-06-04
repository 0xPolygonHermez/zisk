#!/usr/bin/env bash
# Proves the asm-binary cache key tracks its inputs: mutate each input that the
# asm binaries depend on, and check that the cache filename `cargo-zisk` would
# compute actually changes (so a stale binary can't be reused).
#
#   ./tools/test_asm_cache_invalidation.sh        # local: SKIPs absent inputs
#   ./tools/test_asm_cache_invalidation.sh --ci    # CI: build libziskc.a; absent = FAIL
#
# Exit 0 = every mutation changed the name (the fix works).
# Exit 1 = some mutation left the name unchanged (input NOT tracked by the key),
#          or (with --ci) a required input was absent.
# Running it with the rom-setup cache fix reverted reproduces the original bug
# (all three report UNCHANGED).
#
# The probe is rom-setup/examples/cache_probe.rs. `cargo run` rebuilds when an
# input changed, so the build-time hashes (float / transpiler) refresh; the lib
# hash is read at runtime. Every mutated file is restored on exit.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

CI=0
[ "${1:-}" = "--ci" ] && CI=1

PROBE='cargo run -q -p rom-setup --example cache_probe'
LIBZISKC=lib-c/c/lib/libziskc.a
FLOAT=lib-float/c/lib/ziskfloat.elf
TRANSPILER=core/src/zisk_rom_2_asm.rs

pass=0; fail=0
note() { printf '%-26s before=%s after=%s  ' "$1" "$2" "$3"; }
expect_change() { if [ "$2" != "$3" ]; then echo "CHANGED ✅"; pass=$((pass+1)); else echo "UNCHANGED ❌"; fail=$((fail+1)); fi; }
# An absent input is a SKIP locally but a hard FAIL under --ci (so a green CI run
# means every input was genuinely exercised).
missing() { if [ "$CI" = 1 ]; then echo "MISSING ❌ ($1)"; fail=$((fail+1)); else echo "SKIP ($1)"; fi; }

# In CI, build the C lib so its mutation step runs instead of skipping. `lib-c`
# isn't in rom-setup's dep tree, so the probe build alone won't produce it.
if [ "$CI" = 1 ]; then
  echo "[--ci] building lib-c to produce libziskc.a ..."
  cargo build -q -p lib-c || { echo "lib-c build failed"; exit 2; }
fi

# Snapshot files we mutate so we can restore exactly (binaries can't be git-checkout'd if untracked).
tmp=$(mktemp -d)
cp "$LIBZISKC" "$tmp/libziskc.a" 2>/dev/null
cp "$FLOAT"    "$tmp/ziskfloat.elf" 2>/dev/null
cp "$TRANSPILER" "$tmp/transpiler.rs"
restore() {
  [ -f "$tmp/libziskc.a" ]    && cp "$tmp/libziskc.a" "$LIBZISKC"
  [ -f "$tmp/ziskfloat.elf" ] && cp "$tmp/ziskfloat.elf" "$FLOAT"
  cp "$tmp/transpiler.rs" "$TRANSPILER"
}
trap 'restore; rm -rf "$tmp"' EXIT

base=$($PROBE) || { echo "probe build failed"; exit 2; }
echo "baseline name: $base"
echo

# 1. Linked C lib change (runtime hash; no rebuild needed).
if [ -f "$LIBZISKC" ]; then
  printf '\x00probe' >> "$LIBZISKC"
  after=$($PROBE); restore
  note "libziskc.a change" "$base" "$after"; expect_change "x" "$base" "$after"
else
  printf '%-26s ' "libziskc.a change"; missing "not built; run a workspace build first"
fi

# 2. Embedded float ELF change (build-time hash; rebuild picks it up).
if [ -f "$FLOAT" ]; then
  printf '\x00probe' >> "$FLOAT"
  after=$($PROBE); restore
  note "ziskfloat.elf change" "$base" "$after"; expect_change "x" "$base" "$after"
else
  printf '%-26s ' "ziskfloat.elf change"; missing "not present"
fi

# 3. Transpiler source change (build-time hash; rebuild picks it up).
printf '\n// cache-probe %s\n' "$RANDOM" >> "$TRANSPILER"
after=$($PROBE); restore
note "transpiler source change" "$base" "$after"; expect_change "x" "$base" "$after"

echo
echo "PASS (changed): $pass    FAIL (unchanged): $fail"
# On the fix branch we expect all to change. Exit non-zero if any did NOT.
[ "$fail" -eq 0 ]
