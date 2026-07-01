#!/usr/bin/env bash
#
# Build libziskos_staticlib.a with fat LTO and isolate its allocator from the
# host application's allocator.
#
# Why: a `staticlib` crate-type is REQUIRED to register a `#[global_allocator]`,
# which emits the program-global symbols `__rust_alloc`, `__rust_dealloc`, ...
# When libziskos.a is linked into a Rust host those references would bind to the
# host's allocator (ziskos would silently stop using its own `_heap_*`).
# There is no language-level "per-library global allocator", so we isolate at the
# symbol level: after the build we make ziskos's allocator symbols STB_LOCAL, so
# ziskos's `Vec`/`Box` resolve to ITS allocator and the host (C or Rust) keeps
# using its own. The closed symbol set comes from ziskos's default self-contained
# bump allocator: the `staticlib` feature pulls in no external allocator (see
# ziskos Cargo.toml), so the only source of `__rust_alloc` & friends is that
# bump allocator's global-allocator shim.
#
# Option B: the `fat` LTO build leaves LLVM bitcode in the archive, so we first
# finalize it into a single native relocatable object with a partial link
# (`ld.lld -r`) before localizing the symbols with llvm-objcopy.
#
# Usage: .github/scripts/build-libziskos-isolated.sh
# Env overrides: ZISK_TARGET, ZISK_TOOLCHAIN, ZISK_FEATURES
#   ZISK_FEATURES: space/comma-separated cargo features to enable, e.g.
#                  ZISK_FEATURES=alloc-stats .github/scripts/build-libziskos-isolated.sh
set -euo pipefail

. "$HOME/.cargo/env" 2>/dev/null || true

TARGET="${ZISK_TARGET:-riscv64ima-zisk-zkvm-elf}"
TOOLCHAIN="${ZISK_TOOLCHAIN:-zisk}"

FEATURE_ARGS=()
if [[ -n "${ZISK_FEATURES:-}" ]]; then
  FEATURE_ARGS=(--features "$ZISK_FEATURES")
fi

# Rust global-allocator shim symbols. Fat LTO + the partial link below normally
# already internalize these (the shim is only referenced inside ziskos), but if
# any survive as GLOBAL we localize them. This is the full set the shim can emit.
ALLOC_SYMS=(
  __rust_alloc
  __rust_dealloc
  __rust_realloc
  __rust_alloc_zeroed
  __rust_alloc_error_handler
  __rust_alloc_error_handler_should_panic
  __rust_no_alloc_shim_is_unstable
  # ziskos's bump-heap reset helpers. They are `#[no_mangle]` so the staticlib
  # wrappers can bind to them by name (ziskos's `mod alloc` is private, so they
  # are not reachable by Rust path). The partial link above already resolves
  # those cross-crate references, so we localize them here to keep them out of
  # the archive's public symbol surface. (`get_max_used_sys_alloc` is NOT listed:
  # it is the host-facing query API and must stay global.)
  reset_sys_alloc
  update_max_used_sys_alloc
)

# The GlobalAlloc entry points whose isolation we hard-assert at the end: none of
# these may be GLOBAL (would clash with / export to the host) nor UNDEFINED
# (would bind to — and leak into — the host application's allocator).
LEAK_SYMS=(
  __rust_alloc
  __rust_dealloc
  __rust_realloc
  __rust_alloc_zeroed
)

# Public symbols that MUST stay global (sanity check at the end).
REQUIRED_GLOBAL=(
  _start
  read_input
  write_output
  zkvm_init
  zkvm_deinit
  zkvm_keccak256
  zkvm_sha256
  zkvm_secp256k1_ecrecover
  zkvm_secp256k1_verify
  zkvm_secp256r1_verify
  zkvm_bn254_g1_add
  zkvm_bn254_g1_mul
  zkvm_bn254_pairing
  zkvm_bls12_g1_add
  zkvm_bls12_g1_msm
  zkvm_bls12_g2_add
  zkvm_bls12_g2_msm
  zkvm_bls12_pairing
  zkvm_bls12_map_fp_to_g1
  zkvm_bls12_map_fp2_to_g2
  zkvm_blake2f
  zkvm_ripemd160
  zkvm_modexp
  zkvm_kzg_point_eval
)

echo ">> Building ziskos-staticlib (fat LTO) for $TARGET"
cargo "+${TOOLCHAIN}" build -p ziskos-staticlib --release \
  --target "$TARGET" \
  "${FEATURE_ARGS[@]}" \
  --config 'profile.release.lto="fat"'

LIB=$(find target -name "libziskos_staticlib.a" -path "*$TARGET*" | head -1)
if [[ -z "$LIB" ]]; then
  echo "FAIL: libziskos_staticlib.a not found" >&2
  exit 1
fi
echo ">> Archive: $LIB"

# Locate LLVM tooling inside the zisk toolchain sysroot (avoids host LLVM
# version mismatches when reading bitcode produced by fat LTO).
rustup component add llvm-tools --toolchain "$TOOLCHAIN" >/dev/null 2>&1 || true
SYSROOT=$(rustup run "$TOOLCHAIN" rustc --print sysroot)
find_tool() { find "$SYSROOT" -name "$1" -type f 2>/dev/null | head -1; }

LLD=$(find_tool "ld.lld")
LLVM_NM=$(find_tool "llvm-nm")
LLVM_OBJCOPY=$(find_tool "llvm-objcopy")
LLVM_AR=$(find_tool "llvm-ar")
for pair in "ld.lld:$LLD" "llvm-nm:$LLVM_NM" "llvm-objcopy:$LLVM_OBJCOPY" "llvm-ar:$LLVM_AR"; do
  name="${pair%%:*}"; path="${pair#*:}"
  if [[ -z "$path" ]]; then
    echo "FAIL: $name not found in zisk toolchain sysroot ($SYSROOT)" >&2
    exit 1
  fi
done

# 0. The guest is no_std: the raw archive must not bundle std object files.
#    Checked on the raw archive (before the partial link collapses every member
#    into a single combined object, which would hide per-crate member names).
echo ">> Checking no std is bundled"
if "$LLVM_AR" t "$LIB" | grep -q std; then
  echo "FAIL: libziskos_staticlib.a contains std object files:" >&2
  "$LLVM_AR" t "$LIB" | grep std >&2
  exit 1
fi
echo "   OK: no std object files bundled"

COMBINED="${LIB%.a}-combined.o"

# 1. Partial link: finalize fat-LTO bitcode into ONE native relocatable object.
#    --whole-archive pulls every member (with -r there are no undefined refs to
#    drive extraction otherwise). LLD runs the LTO pipeline and emits a native .o.
echo ">> Finalizing LTO bitcode -> $COMBINED (partial link)"
"$LLD" -r -m elf64lriscv --whole-archive "$LIB" -o "$COMBINED"

# Sanity: the combined object must be a real native object readable by llvm-nm.
if ! "$LLVM_NM" "$COMBINED" >/dev/null 2>&1; then
  echo "FAIL: $COMBINED is not a readable native object (LTO finalization failed)" >&2
  exit 1
fi

# 2. Localize any allocator shim symbol that survived as GLOBAL. In nm output a
#    GLOBAL defined symbol has an uppercase type letter; 'U' (undefined) is also
#    uppercase but handled by the leak assertion below, so exclude it here.
NM_OUT=$("$LLVM_NM" "$COMBINED")
LOCALIZE_ARGS=()
for sym in "${ALLOC_SYMS[@]}"; do
  if grep -qE " [A-TV-Z] ${sym}$" <<< "$NM_OUT"; then
    LOCALIZE_ARGS+=("--localize-symbol=${sym}")
    echo "   localizing still-global symbol: ${sym}"
  fi
done
if [[ ${#LOCALIZE_ARGS[@]} -gt 0 ]]; then
  "$LLVM_OBJCOPY" "${LOCALIZE_ARGS[@]}" "$COMBINED"
else
  echo "   allocator already internalized by LTO (no global shim symbols)"
fi

# 3. Repackage: replace the archive with the single localized object.
echo ">> Repackaging $LIB"
rm -f "$LIB"
"$LLVM_AR" crs "$LIB" "$COMBINED"
rm -f "$COMBINED"

# 4. Verify isolation invariant: no GlobalAlloc entry point may be GLOBAL
#    (clash/export) or UNDEFINED (would bind to the host allocator), and the
#    public C-ABI symbols must still be global.
echo ">> Verifying"
FINAL_NM=$("$LLVM_NM" "$LIB")
FAIL=0
for sym in "${LEAK_SYMS[@]}"; do
  if grep -qE " [A-TV-Z] ${sym}$" <<< "$FINAL_NM"; then
    echo "FAIL: $sym is GLOBAL — would clash with / export to the host allocator" >&2
    FAIL=1
  elif grep -qE " U ${sym}$" <<< "$FINAL_NM"; then
    echo "FAIL: $sym is UNDEFINED — would bind to (leak into) the host allocator" >&2
    FAIL=1
  else
    echo "   OK isolated: $sym"
  fi
done
for sym in "${REQUIRED_GLOBAL[@]}"; do
  if ! grep -qE " T ${sym}$" <<< "$FINAL_NM"; then
    echo "FAIL: required public symbol $sym is missing or not global" >&2
    FAIL=1
  fi
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo ">> OK: $LIB built with an allocator isolated from the host application"
