/*
 * zkvm_u256.h — Ethereum Foundation zkVM U256 arithmetic accelerator C
 * interface. Mirrors the standard at:
 *   github.com/eth-act/zkevm-standards/standards/c-interface-accelerators/zkvm_u256.h
 *
 * 256-bit unsigned-integer (EVM word) operations. Every value is a 32-byte
 * BIG-ENDIAN array (EVM word encoding); zkvm_u256 reuses zkvm_bytes_32 from
 * zkvm_accelerators.h. The result pointer MAY alias any input pointer. Division
 * by zero and addmod/mulmod with a zero modulus return zero (EVM semantics).
 *
 * ZisK provides two implementations of the same ABI, chosen at compile time:
 *   - default: each function is a zkvmcall thunk (src/zkvm_calls.s) that the
 *     transpiler turns into a jump to the hand-written `ziskasm_zkvm_u256_*`
 *     routine (ziskasm/zisklib/zkvm/u256.zisk);
 *   - with ZKVM_U256_INLINE defined before including this header (RISC-V builds
 *     only): static inline definitions from zkvm_u256_inline.h, for everything
 *     but the division family (div, mod, divmod, sdiv, smod, sdivmod), which
 *     stays a call.
 * Guest code is the same either way.
 */
#ifndef ZKVM_U256_H
#define ZKVM_U256_H

#include "zkvm_accelerators.h"

#ifdef __cplusplus
extern "C" {
#endif

/* 256-bit unsigned integer, stored as 32 bytes big-endian. */
typedef zkvm_bytes_32 zkvm_u256;

#if defined(ZKVM_U256_INLINE) && defined(__riscv)
#include "zkvm_u256_inline.h"
#define ZKVM_U256_HAVE_INLINE
#endif

/* ---- arithmetic -------------------------------------------------------- */
#ifndef ZKVM_U256_HAVE_INLINE
zkvm_status zkvm_u256_add(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_sub(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_mul(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
#endif
zkvm_status zkvm_u256_div(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* quotient);
zkvm_status zkvm_u256_mod(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* remainder);
zkvm_status zkvm_u256_divmod(const zkvm_u256* a, const zkvm_u256* b,
                             zkvm_u256* quotient, zkvm_u256* remainder);
#ifndef ZKVM_U256_HAVE_INLINE
zkvm_status zkvm_u256_addmod(const zkvm_u256* a, const zkvm_u256* b,
                             const zkvm_u256* n, zkvm_u256* result);
zkvm_status zkvm_u256_mulmod(const zkvm_u256* a, const zkvm_u256* b,
                             const zkvm_u256* n, zkvm_u256* result);
zkvm_status zkvm_u256_exp(const zkvm_u256* base, const zkvm_u256* exponent, zkvm_u256* result);
#endif

/* ---- signed arithmetic (two's complement) ------------------------------ */
zkvm_status zkvm_u256_sdiv(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* quotient);
zkvm_status zkvm_u256_smod(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* remainder);
zkvm_status zkvm_u256_sdivmod(const zkvm_u256* a, const zkvm_u256* b,
                              zkvm_u256* quotient, zkvm_u256* remainder);

#ifndef ZKVM_U256_HAVE_INLINE
/* ---- comparisons (result is 0 or 1) ------------------------------------ */
zkvm_status zkvm_u256_lt(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_gt(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_slt(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_sgt(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_eq(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_iszero(const zkvm_u256* a, zkvm_u256* result);

/* ---- bitwise ----------------------------------------------------------- */
zkvm_status zkvm_u256_and(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_or(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_xor(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_not(const zkvm_u256* a, zkvm_u256* result);
zkvm_status zkvm_u256_byte(const zkvm_u256* i, const zkvm_u256* a, zkvm_u256* result);
zkvm_status zkvm_u256_shl(const zkvm_u256* shift, const zkvm_u256* value, zkvm_u256* result);
zkvm_status zkvm_u256_shr(const zkvm_u256* shift, const zkvm_u256* value, zkvm_u256* result);
zkvm_status zkvm_u256_sar(const zkvm_u256* shift, const zkvm_u256* value, zkvm_u256* result);

/* ---- extended ---------------------------------------------------------- */
zkvm_status zkvm_u256_signextend(const zkvm_u256* b, const zkvm_u256* value, zkvm_u256* result);
#endif /* !ZKVM_U256_HAVE_INLINE */

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZKVM_U256_H */
