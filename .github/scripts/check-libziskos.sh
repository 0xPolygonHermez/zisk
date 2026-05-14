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

# Use llvm-nm to read symbols from the LLVM bitcode archive produced by lto=fat.
# Use a herestring (<<<) instead of echo | grep to avoid a pipefail/SIGPIPE race
# where grep -q exits early, echo receives SIGPIPE (141), and pipefail surfaces
# the 141 rather than grep's 0, flipping the ! check.
NM_OUTPUT=$(llvm-nm "$LIBZISKOS" 2>/dev/null)

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
fi

exit "$FAIL"
