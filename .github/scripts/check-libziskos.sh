#!/usr/bin/env bash
set -euo pipefail

. "$HOME/.cargo/env"

FAIL=0
TARGET=riscv64ima-zisk-zkvm-elf

# Release+LTO: fat LTO gives the compiler full optimisation visibility.
# The resulting archive is the production artifact.
cargo +zisk build -p ziskos-staticlib \
  --target "$TARGET" \
  --release \
  --config 'profile.release.debug=false' \
  --config 'profile.release.lto="fat"'

LIBZISKOS="target/$TARGET/release/libziskos_staticlib.a"
if [ ! -f "$LIBZISKOS" ]; then
  echo "FAIL: libziskos_staticlib.a not found"
  exit 1
fi

# Check that std is not bundled
if ar t "$LIBZISKOS" | grep -q std; then
  echo "FAIL: libziskos_staticlib.a contains std object files:"
  ar t "$LIBZISKOS" | grep std
  FAIL=1
else
  echo "OK: no std object files bundled"
fi

# Use llvm-nm from the zisk toolchain sysroot for consistent symbol output.
rustup component add llvm-tools --toolchain zisk 2>/dev/null || true
LLVM_NM=$(rustup run zisk sh -c 'find "$(rustc --print sysroot)" -name "llvm-nm" -type f | head -1')
if [[ -z "$LLVM_NM" ]]; then
  echo "FAIL: llvm-nm not found in zisk toolchain sysroot"
  exit 1
fi
echo "Using llvm-nm: $LLVM_NM"

NM_OUTPUT=$("$LLVM_NM" "$LIBZISKOS" 2>/dev/null || true)
if [[ -z "$NM_OUTPUT" ]]; then
  echo "FAIL: llvm-nm produced no output for $LIBZISKOS"
  exit 1
fi

REQUIRED_SYMBOLS=(
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

for SYM in "${REQUIRED_SYMBOLS[@]}"; do
  if ! grep -qE " T $SYM$" <<< "$NM_OUTPUT"; then
    echo "FAIL: missing symbol: $SYM"
    FAIL=1
  fi
done

# Symbols used in the link test: all of REQUIRED_SYMBOLS except _start
# (_start is provided by the archive itself, not declared as extern in main.rs)
LINK_SYMBOLS=()
for SYM in "${REQUIRED_SYMBOLS[@]}"; do
  [[ "$SYM" != "_start" ]] && LINK_SYMBOLS+=("$SYM")
done

# ── Link test: verify no global allocator call reaches the C consumer ──────────
# Build a minimal no_std Cargo binary that provides main() and calls every
# exported symbol.  The archive supplies _start (which calls main), the zisk
# toolchain supplies the linker script and all platform symbols.
#
# If every exported function is allocation-free the binary links cleanly.
# If any function transitively reaches __rust_alloc, ForbiddenAlloc's body
# references __ziskos_forbidden_alloc — an intentionally undefined symbol —
# and the linker fails with a clear "undefined symbol" error.
LINK_TMPDIR=$(mktemp -d)
mkdir -p "$LINK_TMPDIR/src"

# Absolute path so the build.rs works regardless of working directory.
LIBZISKOS_DIR=$(dirname "$(realpath "$LIBZISKOS")")

cat > "$LINK_TMPDIR/Cargo.toml" << 'CARGO_EOF'
[package]
name = "ziskos_link_test"
version = "0.1.0"
edition = "2021"
CARGO_EOF

# build.rs: link against the pre-built release archive.
cat > "$LINK_TMPDIR/build.rs" << BUILD_RS_EOF
fn main() {
    println!("cargo:rustc-link-search=${LIBZISKOS_DIR}");
    println!("cargo:rustc-link-lib=static=ziskos_staticlib");
}
BUILD_RS_EOF

# main.rs: no_std binary; main() is the user entry point, _start comes from
# the archive.  Wrong argument types are fine — the linker only resolves names.
{
  printf '#![no_std]\n#![no_main]\n#![allow(improper_ctypes)]\n\nextern "C" {\n'
  for SYM in "${LINK_SYMBOLS[@]}"; do
    printf '    fn %s();\n' "$SYM"
  done
  printf '}\n\n#[no_mangle]\npub extern "C" fn main() -> i32 {\n    unsafe {\n'
  for SYM in "${LINK_SYMBOLS[@]}"; do
    printf '        %s();\n' "$SYM"
  done
  printf '    }\n    0\n}\n\n#[panic_handler]\nfn panic(_: &core::panic::PanicInfo) -> ! { loop {} }\n'
} > "$LINK_TMPDIR/src/main.rs"

cat "$LINK_TMPDIR/src/main.rs"

LINK_TEST_OUTPUT=$(cargo +zisk build \
  --manifest-path "$LINK_TMPDIR/Cargo.toml" \
  --target "$TARGET" \
  2>&1 || true)

if echo "$LINK_TEST_OUTPUT" | grep -q "__ziskos_forbidden_alloc"; then
  echo "FAIL: link test — global allocator reachable from exported symbols:"
  echo "$LINK_TEST_OUTPUT" | grep "__ziskos_forbidden_alloc"
  FAIL=1
elif echo "$LINK_TEST_OUTPUT" | grep -qE "^error"; then
  echo "FAIL: link test failed unexpectedly:"
  echo "$LINK_TEST_OUTPUT"
  FAIL=1
else
  echo "OK: link test passed — executable linked without errors"
fi

rm -rf "$LINK_TMPDIR"

if [ "$FAIL" -eq 0 ]; then
  echo "OK: libziskos_staticlib.a passes all checks"
else
  exit "$FAIL"
fi
