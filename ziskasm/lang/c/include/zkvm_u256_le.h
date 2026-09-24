/*
 * zkvm_u256_le.h — a little-endian variant of the EF U256 ABI (zkvm_u256.h).
 *
 * NOT part of the EF standard: a ZisK proposal. Same 27 operations, same EVM
 * semantics (division by zero and addmod/mulmod with a zero modulus return zero,
 * out-of-range shifts saturate), results may alias inputs, and every function
 * returns ZKVM_EOK. The one difference is the value layout: four 64-bit limbs,
 * least significant first, which is how EVM interpreters (evmone's intx, revm's
 * ruint, ziskethone's zevm) already keep their stack words and what the ZisK
 * 256-bit precompiles consume. So there is no byte reversal on either side.
 *
 * Two implementations of the same ABI, chosen at compile time (as for zkvm_u256.h):
 *   - default: every function is a zkvmcall thunk (src/zkvm_calls.s) that the
 *     transpiler turns into a jump to a .zisk routine
 *     (ziskasm/zisklib/zkvm/u256_le.zisk), which calls the uint256 cores on the
 *     argument pointers directly;
 *   - with ZKVM_U256_LE_INLINE defined before including this header (RISC-V
 *     builds only): static inline definitions for everything but the division
 *     family (div, mod, divmod, sdiv, smod, sdivmod), most of them a single
 *     precompile on the operands in place (add256 for add/sub, arith256 for
 *     mul/exp, arith256_mod for addmod/mulmod). The division family needs the
 *     division hint, so it stays a call.
 * Guest code is the same either way.
 */
#ifndef ZKVM_U256_LE_H
#define ZKVM_U256_LE_H

#include "zkvm_accelerators.h"

#ifdef __cplusplus
extern "C" {
#endif

/* 256-bit unsigned integer as four 64-bit limbs, limbs[0] least significant. */
typedef struct { uint64_t limbs[4]; } zkvm_u256_le;

/* ---- division family: always a call ------------------------------------------ */
zkvm_status zkvm_u256_le_div(const zkvm_u256_le* a, const zkvm_u256_le* b,
                             zkvm_u256_le* quotient);
zkvm_status zkvm_u256_le_mod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                             zkvm_u256_le* remainder);
zkvm_status zkvm_u256_le_divmod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                zkvm_u256_le* quotient, zkvm_u256_le* remainder);
zkvm_status zkvm_u256_le_sdiv(const zkvm_u256_le* a, const zkvm_u256_le* b,
                              zkvm_u256_le* quotient);
zkvm_status zkvm_u256_le_smod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                              zkvm_u256_le* remainder);
zkvm_status zkvm_u256_le_sdivmod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                 zkvm_u256_le* quotient, zkvm_u256_le* remainder);

#if !(defined(ZKVM_U256_LE_INLINE) && defined(__riscv))

/* ---- default: calls ------------------------------------------------------------ */
zkvm_status zkvm_u256_le_add(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_sub(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_mul(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_addmod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                const zkvm_u256_le* n, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_mulmod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                const zkvm_u256_le* n, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_exp(const zkvm_u256_le* base, const zkvm_u256_le* exponent,
                             zkvm_u256_le* result);
zkvm_status zkvm_u256_le_lt(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_gt(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_slt(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_sgt(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_eq(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_iszero(const zkvm_u256_le* a, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_and(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_or(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_xor(const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_not(const zkvm_u256_le* a, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_byte(const zkvm_u256_le* i, const zkvm_u256_le* a, zkvm_u256_le* result);
zkvm_status zkvm_u256_le_shl(const zkvm_u256_le* shift, const zkvm_u256_le* value,
                             zkvm_u256_le* result);
zkvm_status zkvm_u256_le_shr(const zkvm_u256_le* shift, const zkvm_u256_le* value,
                             zkvm_u256_le* result);
zkvm_status zkvm_u256_le_sar(const zkvm_u256_le* shift, const zkvm_u256_le* value,
                             zkvm_u256_le* result);
zkvm_status zkvm_u256_le_signextend(const zkvm_u256_le* b, const zkvm_u256_le* value,
                                    zkvm_u256_le* result);

#else /* ZKVM_U256_LE_INLINE */

#define ZKVM_U256_LE_I static inline __attribute__((always_inline))

/* ---- helpers --------------------------------------------------------------- */

/* add256: c = a + b + cin (cin is 0 or 1), returns the carry out. */
ZKVM_U256_LE_I uint64_t zkvm_u256_le_i_add256(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                              uint64_t cin, zkvm_u256_le* c) {
    const uint64_t p[4] = {(uint64_t)(uintptr_t)a, (uint64_t)(uintptr_t)b, cin,
                           (uint64_t)(uintptr_t)c};
    uint64_t carry;
    __asm__ volatile("csrrs %0, 0x811, %1" : "=r"(carry) : "r"(p) : "memory");
    return carry;
}
/* arith256: a * b + c = dh:dl. */
ZKVM_U256_LE_I void zkvm_u256_le_i_arith256(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                            const zkvm_u256_le* c, zkvm_u256_le* dl,
                                            zkvm_u256_le* dh) {
    const void* p[5] = {a, b, c, dl, dh};
    __asm__ volatile("csrs 0x801, %0" : : "r"(p) : "memory");
}
/* arith256_mod: d = (a * b + c) mod m, m != 0. */
ZKVM_U256_LE_I void zkvm_u256_le_i_arith256_mod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                                const zkvm_u256_le* c, const zkvm_u256_le* m,
                                                zkvm_u256_le* d) {
    const void* p[5] = {a, b, c, m, d};
    __asm__ volatile("csrs 0x802, %0" : : "r"(p) : "memory");
}
ZKVM_U256_LE_I int zkvm_u256_le_i_is_zero(const zkvm_u256_le* x) {
    return (x->limbs[0] | x->limbs[1] | x->limbs[2] | x->limbs[3]) == 0;
}
ZKVM_U256_LE_I void zkvm_u256_le_i_set_small(zkvm_u256_le* r, uint64_t v) {
    r->limbs[0] = v;
    r->limbs[1] = r->limbs[2] = r->limbs[3] = 0;
}
/* A shift/index/byte operand below `limit`, as a plain number; `limit` if not. */
ZKVM_U256_LE_I uint64_t zkvm_u256_le_i_small(const zkvm_u256_le* x, uint64_t limit) {
    return (x->limbs[1] | x->limbs[2] | x->limbs[3]) == 0 && x->limbs[0] < limit
               ? x->limbs[0] : limit;
}
ZKVM_U256_LE_I int zkvm_u256_le_i_ult(const uint64_t a[4], const uint64_t b[4]) {
    _Pragma("GCC unroll 4")
    for (int i = 3; i >= 0; i--)
        if (a[i] != b[i]) return a[i] < b[i];
    return 0;
}
ZKVM_U256_LE_I int zkvm_u256_le_i_slt(const uint64_t a[4], const uint64_t b[4]) {
    const uint64_t s = 1ull << 63;
    const uint64_t x[4] = {a[0], a[1], a[2], a[3] ^ s}, y[4] = {b[0], b[1], b[2], b[3] ^ s};
    return zkvm_u256_le_i_ult(x, y);
}
/* z = x >> s for s < 256, shifting in `fill` (0, or all ones for sar). */
ZKVM_U256_LE_I void zkvm_u256_le_i_shr(const uint64_t x[4], unsigned s, uint64_t fill,
                                       uint64_t z[4]) {
    unsigned q = s / 64, b = s % 64;
    _Pragma("GCC unroll 4")
    for (unsigned i = 0; i < 4; i++) {
        uint64_t lo = i + q < 4 ? x[i + q] : fill;
        uint64_t hi = i + q + 1 < 4 ? x[i + q + 1] : fill;
        z[i] = b ? (lo >> b) | (hi << (64 - b)) : lo;
    }
}

/* ---- arithmetic ------------------------------------------------------------ */

ZKVM_U256_LE_I zkvm_status zkvm_u256_le_add(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                            zkvm_u256_le* result) {
    zkvm_u256_le_i_add256(a, b, 0, result);
    return ZKVM_EOK;
}
/* a - b = a + ~b + 1 */
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_sub(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                            zkvm_u256_le* result) {
    zkvm_u256_le nb;
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) nb.limbs[i] = ~b->limbs[i];
    zkvm_u256_le_i_add256(a, &nb, 1, result);
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_mul(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                            zkvm_u256_le* result) {
    static const zkvm_u256_le zero = {{0, 0, 0, 0}};
    zkvm_u256_le hi;
    zkvm_u256_le_i_arith256(a, b, &zero, result, &hi);
    return ZKVM_EOK;
}
/* (a * 1 + b) mod n */
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_addmod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                               const zkvm_u256_le* n, zkvm_u256_le* result) {
    static const zkvm_u256_le one = {{1, 0, 0, 0}};
    if (zkvm_u256_le_i_is_zero(n)) zkvm_u256_le_i_set_small(result, 0);
    else zkvm_u256_le_i_arith256_mod(a, &one, b, n, result);
    return ZKVM_EOK;
}
/* (a * b + 0) mod n */
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_mulmod(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                               const zkvm_u256_le* n, zkvm_u256_le* result) {
    static const zkvm_u256_le zero = {{0, 0, 0, 0}};
    if (zkvm_u256_le_i_is_zero(n)) zkvm_u256_le_i_set_small(result, 0);
    else zkvm_u256_le_i_arith256_mod(a, b, &zero, n, result);
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_exp(const zkvm_u256_le* base,
                                            const zkvm_u256_le* exponent,
                                            zkvm_u256_le* result) {
    static const zkvm_u256_le zero = {{0, 0, 0, 0}};
    zkvm_u256_le b = *base, e = *exponent, acc = {{1, 0, 0, 0}}, hi;
    int top = 255;                                     /* skip the exponent's leading zeros */
    while (top >= 0 && !((e.limbs[top / 64] >> (top % 64)) & 1)) top--;
    for (int i = top; i >= 0; i--) {                   /* left-to-right square-and-multiply */
        zkvm_u256_le_i_arith256(&acc, &acc, &zero, &acc, &hi);
        if ((e.limbs[i / 64] >> (i % 64)) & 1) zkvm_u256_le_i_arith256(&acc, &b, &zero, &acc, &hi);
    }
    *result = acc;
    return ZKVM_EOK;
}

/* ---- comparisons ----------------------------------------------------------- */

ZKVM_U256_LE_I zkvm_status zkvm_u256_le_lt(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                           zkvm_u256_le* result) {
    zkvm_u256_le_i_set_small(result, zkvm_u256_le_i_ult(a->limbs, b->limbs));
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_gt(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                           zkvm_u256_le* result) {
    zkvm_u256_le_i_set_small(result, zkvm_u256_le_i_ult(b->limbs, a->limbs));
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_slt(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                            zkvm_u256_le* result) {
    zkvm_u256_le_i_set_small(result, zkvm_u256_le_i_slt(a->limbs, b->limbs));
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_sgt(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                            zkvm_u256_le* result) {
    zkvm_u256_le_i_set_small(result, zkvm_u256_le_i_slt(b->limbs, a->limbs));
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_eq(const zkvm_u256_le* a, const zkvm_u256_le* b,
                                           zkvm_u256_le* result) {
    uint64_t d = (a->limbs[0] ^ b->limbs[0]) | (a->limbs[1] ^ b->limbs[1]) |
                 (a->limbs[2] ^ b->limbs[2]) | (a->limbs[3] ^ b->limbs[3]);
    zkvm_u256_le_i_set_small(result, d == 0);
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_iszero(const zkvm_u256_le* a, zkvm_u256_le* result) {
    zkvm_u256_le_i_set_small(result, zkvm_u256_le_i_is_zero(a));
    return ZKVM_EOK;
}

/* ---- bitwise --------------------------------------------------------------- */

#define ZKVM_U256_LE_I_BITWISE(name, op)                                                  \
    ZKVM_U256_LE_I zkvm_status zkvm_u256_le_##name(const zkvm_u256_le* a,                 \
                                                   const zkvm_u256_le* b,                 \
                                                   zkvm_u256_le* result) {                \
        zkvm_u256_le r;                                                                   \
        _Pragma("GCC unroll 4")                                                           \
        for (int i = 0; i < 4; i++) r.limbs[i] = a->limbs[i] op b->limbs[i];              \
        *result = r;                                                                      \
        return ZKVM_EOK;                                                                  \
    }
ZKVM_U256_LE_I_BITWISE(and, &)
ZKVM_U256_LE_I_BITWISE(or, |)
ZKVM_U256_LE_I_BITWISE(xor, ^)
#undef ZKVM_U256_LE_I_BITWISE

ZKVM_U256_LE_I zkvm_status zkvm_u256_le_not(const zkvm_u256_le* a, zkvm_u256_le* result) {
    zkvm_u256_le r;
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) r.limbs[i] = ~a->limbs[i];
    *result = r;
    return ZKVM_EOK;
}
/* byte(i, a): byte i counting from the most significant (EVM BYTE). */
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_byte(const zkvm_u256_le* i, const zkvm_u256_le* a,
                                             zkvm_u256_le* result) {
    uint64_t k = zkvm_u256_le_i_small(i, 32);
    uint64_t v = k < 32 ? (a->limbs[(31 - k) / 8] >> (8 * ((31 - k) % 8))) & 0xff : 0;
    zkvm_u256_le_i_set_small(result, v);
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_shl(const zkvm_u256_le* shift, const zkvm_u256_le* value,
                                            zkvm_u256_le* result) {
    const uint64_t* x = value->limbs;
    uint64_t z[4] = {0, 0, 0, 0};
    uint64_t s = zkvm_u256_le_i_small(shift, 256);
    if (s < 256) {
        int q = (int)(s / 64);
        unsigned b = s % 64;
        _Pragma("GCC unroll 4")
        for (int i = 3; i >= 0; i--) {
            uint64_t hi = i - q >= 0 ? x[i - q] : 0;
            uint64_t lo = i - q - 1 >= 0 ? x[i - q - 1] : 0;
            z[i] = b ? (hi << b) | (lo >> (64 - b)) : hi;
        }
    }
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) result->limbs[i] = z[i];
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_shr(const zkvm_u256_le* shift, const zkvm_u256_le* value,
                                            zkvm_u256_le* result) {
    uint64_t z[4] = {0, 0, 0, 0};
    uint64_t s = zkvm_u256_le_i_small(shift, 256);
    if (s < 256) zkvm_u256_le_i_shr(value->limbs, (unsigned)s, 0, z);
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) result->limbs[i] = z[i];
    return ZKVM_EOK;
}
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_sar(const zkvm_u256_le* shift, const zkvm_u256_le* value,
                                            zkvm_u256_le* result) {
    uint64_t z[4];
    uint64_t fill = (int64_t)value->limbs[3] < 0 ? ~0ull : 0;
    uint64_t s = zkvm_u256_le_i_small(shift, 256);
    if (s < 256) zkvm_u256_le_i_shr(value->limbs, (unsigned)s, fill, z);
    else z[0] = z[1] = z[2] = z[3] = fill;
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) result->limbs[i] = z[i];
    return ZKVM_EOK;
}

/* ---- extended -------------------------------------------------------------- */

/* signextend(b, value): extend the sign of byte b (0 = least significant). */
ZKVM_U256_LE_I zkvm_status zkvm_u256_le_signextend(const zkvm_u256_le* b,
                                                   const zkvm_u256_le* value,
                                                   zkvm_u256_le* result) {
    zkvm_u256_le x = *value;
    uint64_t k = zkvm_u256_le_i_small(b, 31);
    if (k < 31) {
        unsigned bit = 8 * (unsigned)k + 7, q = bit / 64, s = bit % 64;
        uint64_t fill = (x.limbs[q] >> s) & 1 ? ~0ull : 0;
        uint64_t keep = s == 63 ? ~0ull : (2ull << s) - 1;
        x.limbs[q] = (x.limbs[q] & keep) | (fill & ~keep);
        for (unsigned i = q + 1; i < 4; i++) x.limbs[i] = fill;
    }
    *result = x;
    return ZKVM_EOK;
}

#undef ZKVM_U256_LE_I

#endif /* ZKVM_U256_LE_INLINE */

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZKVM_U256_LE_H */
