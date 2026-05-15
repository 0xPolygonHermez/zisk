#!/usr/bin/env bash
set -euo pipefail

. "$HOME/.cargo/env"

FAIL=0
TARGET=riscv64ima-zisk-zkvm-elf

cargo +zisk build -p ziskos-staticlib --release \
  --target "$TARGET" \
  --config 'profile.release.lto="fat"'

LIBZISKOS=$(find target -name "libziskos_staticlib.a" -path "*$TARGET*" | head -1)
if [ -z "$LIBZISKOS" ]; then
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

# Use llvm-nm from the zisk toolchain sysroot to avoid LLVM version mismatches
# when reading bitcode objects produced by lto="fat".
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

if [ "$FAIL" -eq 0 ]; then
  echo "OK: libziskos_staticlib.a passes all checks"
else
  exit "$FAIL"
fi