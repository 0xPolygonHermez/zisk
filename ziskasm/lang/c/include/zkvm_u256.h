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
 * ZisK provides three builds of the same ABI, chosen at compile time:
 *   - default (RISC-V): the hand-written .zisk routines (ziskasm/zisklib/zkvm/
 *     u256.zisk). The division family and exp are called through zkvmcall thunks
 *     (src/zkvm_calls.s); every other function is an inline zkvmcall (see below);
 *   - with ZKVM_U256_CALLS defined before including this header, or off RISC-V:
 *     every function is only declared, as in the EF standard's header, and is
 *     called through its thunk. An inline zkvmcall's thunk expands the same
 *     routine body (definitions/src/zkvmcall.rs), so it gives the same results;
 *   - with ZKVM_U256_INLINE defined (RISC-V builds only): static inline C
 *     definitions from zkvm_u256_inline.h, for everything but the division
 *     family (div, mod, divmod, sdiv, smod, sdivmod), which stays a call.
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

/* ---- division family: always a call ------------------------------------- */
zkvm_status zkvm_u256_div(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* quotient);
zkvm_status zkvm_u256_mod(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* remainder);
zkvm_status zkvm_u256_divmod(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* quotient,
                             zkvm_u256* remainder);
zkvm_status zkvm_u256_sdiv(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* quotient);
zkvm_status zkvm_u256_smod(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* remainder);
zkvm_status zkvm_u256_sdivmod(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* quotient,
                              zkvm_u256* remainder);

#if defined(ZKVM_U256_INLINE) && defined(__riscv)
#include "zkvm_u256_inline.h"

#elif defined(__riscv) && !defined(ZKVM_U256_CALLS)
/* ---- default: inline zkvmcalls ------------------------------------------ */
/* A `csrs` per argument, each naming the register the compiler picked, which the
 * transpiler replaces by the .zisk routine's body on those registers
 * (definitions/src/zkvmcall.rs). The body writes no RISC-V register, so the
 * compiler is told only which memory is read and written, and the status is the
 * constant ZKVM_EOK, which the compiler folds away. */
#define ZKVM_U256_ZC_BIN(name, id)                                                        \
    static inline __attribute__((always_inline)) zkvm_status zkvm_u256_##name(            \
        const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result) {                      \
        __asm__("csrs " #id ", %1\n\t"                /* zkvmcall, argument 0 */         \
                "csrs 0x8E0, %2\n\t"                  /* argument 1 */                   \
                "csrs 0x8E1, %3"                       /* argument 2 */                   \
                : "=m"(*result) : "r"(a), "r"(b), "r"(result), "m"(*a), "m"(*b));         \
        return ZKVM_EOK;                                                                  \
    }
#define ZKVM_U256_ZC_TER(name, id)                                                        \
    static inline __attribute__((always_inline)) zkvm_status zkvm_u256_##name(            \
        const zkvm_u256* a, const zkvm_u256* b, const zkvm_u256* n, zkvm_u256* result) {  \
        __asm__("csrs " #id ", %1\n\t"                /* zkvmcall, argument 0 */         \
                "csrs 0x8E0, %2\n\t"                  /* argument 1 */                   \
                "csrs 0x8E1, %3\n\t"                  /* argument 2 */                   \
                "csrs 0x8E2, %4"                       /* argument 3 */                   \
                : "=m"(*result)                                                           \
                : "r"(a), "r"(b), "r"(n), "r"(result), "m"(*a), "m"(*b), "m"(*n));        \
        return ZKVM_EOK;                                                                  \
    }
#define ZKVM_U256_ZC_UN(name, id)                                                         \
    static inline __attribute__((always_inline)) zkvm_status zkvm_u256_##name(            \
        const zkvm_u256* a, zkvm_u256* result) {                                          \
        __asm__("csrs " #id ", %1\n\t"                /* zkvmcall, argument 0 */         \
                "csrs 0x8E0, %2"                       /* argument 1 */                   \
                : "=m"(*result) : "r"(a), "r"(result), "m"(*a));                          \
        return ZKVM_EOK;                                                                  \
    }
ZKVM_U256_ZC_BIN(add, 0x865)
ZKVM_U256_ZC_BIN(sub, 0x866)
ZKVM_U256_ZC_BIN(mul, 0x867)
ZKVM_U256_ZC_BIN(lt, 0x871)
ZKVM_U256_ZC_BIN(gt, 0x872)
ZKVM_U256_ZC_BIN(slt, 0x873)
ZKVM_U256_ZC_BIN(sgt, 0x874)
ZKVM_U256_ZC_BIN(eq, 0x875)
ZKVM_U256_ZC_BIN(and, 0x877)
ZKVM_U256_ZC_BIN(or, 0x878)
ZKVM_U256_ZC_BIN(xor, 0x879)
ZKVM_U256_ZC_BIN(byte, 0x87B)
ZKVM_U256_ZC_BIN(shl, 0x87C)
ZKVM_U256_ZC_BIN(shr, 0x87D)
ZKVM_U256_ZC_BIN(sar, 0x87E)
ZKVM_U256_ZC_BIN(signextend, 0x87F)
ZKVM_U256_ZC_TER(addmod, 0x86B)
ZKVM_U256_ZC_TER(mulmod, 0x86C)
ZKVM_U256_ZC_UN(iszero, 0x876)
ZKVM_U256_ZC_UN(not, 0x87A)
#undef ZKVM_U256_ZC_BIN
#undef ZKVM_U256_ZC_TER
#undef ZKVM_U256_ZC_UN
/* Calls. */
zkvm_status zkvm_u256_exp(const zkvm_u256* base, const zkvm_u256* exponent,
                          zkvm_u256* result);

#else
/* ---- declarations only: calls ------------------------------------------- */
zkvm_status zkvm_u256_add(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_sub(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_mul(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_addmod(const zkvm_u256* a, const zkvm_u256* b, const zkvm_u256* n,
                             zkvm_u256* result);
zkvm_status zkvm_u256_mulmod(const zkvm_u256* a, const zkvm_u256* b, const zkvm_u256* n,
                             zkvm_u256* result);
zkvm_status zkvm_u256_exp(const zkvm_u256* base, const zkvm_u256* exponent,
                          zkvm_u256* result);
zkvm_status zkvm_u256_lt(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_gt(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_slt(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_sgt(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_eq(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_iszero(const zkvm_u256* a, zkvm_u256* result);
zkvm_status zkvm_u256_and(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_or(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_xor(const zkvm_u256* a, const zkvm_u256* b, zkvm_u256* result);
zkvm_status zkvm_u256_not(const zkvm_u256* a, zkvm_u256* result);
zkvm_status zkvm_u256_byte(const zkvm_u256* i, const zkvm_u256* a, zkvm_u256* result);
zkvm_status zkvm_u256_shl(const zkvm_u256* shift, const zkvm_u256* value,
                          zkvm_u256* result);
zkvm_status zkvm_u256_shr(const zkvm_u256* shift, const zkvm_u256* value,
                          zkvm_u256* result);
zkvm_status zkvm_u256_sar(const zkvm_u256* shift, const zkvm_u256* value,
                          zkvm_u256* result);
zkvm_status zkvm_u256_signextend(const zkvm_u256* b, const zkvm_u256* value,
                                 zkvm_u256* result);
#endif

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZKVM_U256_H */
