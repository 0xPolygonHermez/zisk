#!/usr/bin/env bash
set -euo pipefail

. "$HOME/.cargo/env"

FAIL=0
TARGET=riscv64ima-zisk-zkvm-elf

# Unoptimised, no LTO, no debug info.
# opt-level=0 keeps every crate as a separate ELF object with intact symbol
# tables; LTO would merge everything into a single bitcode module where
# per-object undefined references are no longer visible to nm, which would
# break the global-allocator check below.  Debug symbols are suppressed to
# keep the artifact small and the nm output clean.
cargo +zisk build -p ziskos-staticlib \
  --target "$TARGET" \
  --config 'profile.dev.debug=false'

LIBZISKOS="target/$TARGET/debug/libziskos_staticlib.a"
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

# Use llvm-nm from the zisk toolchain sysroot for consistent symbol output.
# Standard nm also works for regular ELF objects (no fat LTO bitcode here),
# but llvm-nm is already available and produces identical results.
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

# ── No global-allocator usage in accelerator code ─────────────────────────────
# Accelerator functions must allocate exclusively through the scratch arena
# (BumpScratch), never through the global allocator.
#
# The check looks for "U __rustc::__rust_alloc" — an undefined reference to the
# global allocator entry point — in object files that belong to the ziskos crates.
#
# Why this catches all global-allocator use:
#   The alloc-crate functions that lead to __rust_alloc (Global::allocate,
#   alloc::alloc::alloc, exchange_malloc for Box, …) are all marked #[inline].
#   The Rust compiler emits #[inline] functions into every crate that uses them
#   as LLVM "available_externally" definitions — even at opt-level=0 — so their
#   bodies, which reference __rust_alloc, appear in the caller's object file as
#   undefined (U) symbols.  Any ziskos object that uses the global allocator
#   through any path (Vec<T, Global>, Box, String, Arc, …) therefore carries
#   "U __rustc::__rust_alloc" in its symbol table.
#
# Symbol name:
#   The raw mangled form (_RNvCsd..._7___rustc12___rust_alloc) embeds a
#   build-specific crate hash, so -C (demangle) is required before matching.
#
# Filter:
#   llvm-nm -A -C output format:
#     <archive_path>:<member_name>: <addr> <type> <demangled_symbol>
#   awk -F: isolates field $2 (the member filename).  Members whose name starts
#   with "ziskos" are the project crates; alloc, core, proofman_verifier, serde,
#   num_bigint, and other dependencies that legitimately call __rust_alloc are
#   excluded.
#
# Known limitation — codegen-units=1:
#   Building with codegen-units=1 collapses the entire ziskos crate into a
#   single object file.  That object both defines "T __rustc::__rust_alloc"
#   (the #[global_allocator] wrapper in alloc/bump.rs) and contains all
#   accelerator code.  Any "U __rustc::__rust_alloc" that a violation would
#   produce resolves internally to the definition in the same CGU and therefore
#   does not appear as "U" — the check is completely blind to violations.
#
#   With codegen-units ≥ 2 (Rust's dev-profile default is 256) the allocator
#   definition lands in its own object, separate from the accelerator CGUs, so
#   violations remain visible as "U" symbols.  Do not add
#   --config 'profile.dev.codegen-units=1' to this build.
RUST_ALLOC_DIRECT=$("$LLVM_NM" -C -A "$LIBZISKOS" 2>/dev/null | \
  grep " U __rustc::__rust_alloc" | \
  awk -F: '$2 ~ /^ziskos/' || true)

if [ -n "$RUST_ALLOC_DIRECT" ]; then
  echo "FAIL: __rustc::__rust_alloc referenced from ziskos object files:"
  echo "$RUST_ALLOC_DIRECT"
  FAIL=1
else
  echo "OK: no __rustc::__rust_alloc references from ziskos object files"
fi

if [ "$FAIL" -eq 0 ]; then
  echo "OK: libziskos_staticlib.a passes all checks"
else
  exit "$FAIL"
fi
