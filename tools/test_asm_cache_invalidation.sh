#!/usr/bin/env bash
# Proves the asm-binary cache key tracks its inputs: mutate each input that the
# asm binaries depend on and check that the cache filename `cargo-zisk` would
# compute actually changes (so a stale binary can't be reused).
#
#   ./tools/test_asm_cache_invalidation.sh         # local
#   ./tools/test_asm_cache_invalidation.sh --ci     # CI: absent input = hard FAIL
#
# Exit 0 = every mutation changed the name (the fix works).
# Exit 1 = some mutation left the name unchanged (input NOT tracked by the key),
#          or (with --ci) a required input was absent.
# Running it with the rom-setup cache fix reverted reproduces the original bug
# (every input reports UNCHANGED).
#
# All inputs are hashed at build time (see rom-setup/build.rs), so each mutation
# needs a rebuild — `cargo run` on the cache_probe example does that. The libs
# are hashed by *source* (ziskclib/src, lib-c/c/src), not by their compiled
# archives, so this mutates the sources. Every mutated file is restored on exit.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

CI=0
[ "${1:-}" = "--ci" ] && CI=1

# (label, file) pairs — every input that flows into the asm cache key.
INPUTS=(
  "transpiler source:core/src/zisk_rom_2_asm.rs"
  "emulator-asm source:emulator-asm/src/emu.c"
  "ziskclib source:ziskclib/src/lib.rs"
  "lib-c source:lib-c/c/src/main.cpp"
  "embedded float ELF:lib-float/c/lib/ziskfloat.elf"
)

# Run the probe; abort on a build/run failure so a failed probe can never be
# misread as an empty-string "CHANGED" result.
run_probe() {
  local out
  if ! out=$(cargo run -q -p rom-setup --example cache_probe); then
    echo "probe build/run failed" >&2
    exit 2
  fi
  printf '%s' "$out"
}

pass=0; fail=0
tmp=$(mktemp -d)
CUR_FILE=""; CUR_SNAP=""
cleanup() { [ -n "$CUR_FILE" ] && cp "$CUR_SNAP" "$CUR_FILE"; rm -rf "$tmp"; }
trap cleanup EXIT INT TERM

base=$(run_probe)
echo "baseline name: $base"
echo

i=0
for entry in "${INPUTS[@]}"; do
  label=${entry%%:*}; file=${entry#*:}
  printf '%-22s ' "$label"
  if [ ! -f "$file" ]; then
    if [ "$CI" = 1 ]; then echo "MISSING ❌ ($file)"; fail=$((fail+1)); else echo "SKIP ($file not present)"; fi
    continue
  fi
  # Snapshot, mutate, probe, restore. `//` is a valid comment in Rust/C/C++
  # (the transpiler .rs is compiled during the probe build, so it must stay
  # valid) and harmless trailing bytes for the ELF (never parsed by the probe).
  cp "$file" "$tmp/snap.$i"
  CUR_FILE="$file"; CUR_SNAP="$tmp/snap.$i"   # arm restore-on-exit
  printf '\n// cache-probe %s\n' "$RANDOM" >> "$file"
  after=$(run_probe)
  cp "$tmp/snap.$i" "$file"
  CUR_FILE=""; CUR_SNAP=""                    # disarm: file restored
  if [ "$base" != "$after" ]; then echo "CHANGED ✅"; pass=$((pass+1)); else echo "UNCHANGED ❌"; fail=$((fail+1)); fi
  i=$((i+1))
done

echo
echo "PASS (changed): $pass    FAIL (unchanged): $fail"
[ "$fail" -eq 0 ]
