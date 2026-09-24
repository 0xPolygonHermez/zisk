/*
 * zkvm_u256_inline.h — an inline implementation of the EF U256 ABI (zkvm_u256.h).
 *
 * Don't include this directly: define ZKVM_U256_INLINE and include zkvm_u256.h.
 * The functions keep their standard names, signatures and semantics (32-byte
 * big-endian values, results may alias inputs, EVM semantics for zero moduli and
 * out-of-range shifts), so guest code is the same with either implementation;
 * only the cost changes. Every one of them returns ZKVM_EOK.
 *
 * They load the big-endian operands into little-endian limbs (one rev8 per limb
 * with Zbb), compute in plain C or with the arith256 / arith256_mod precompiles,
 * and store the result back. The division family (div, mod, divmod, sdiv, smod,
 * sdivmod) is not here: it needs the division hint, so it stays a call to the
 * .zisk routines.
 */
#ifndef ZKVM_U256_INLINE_H
#define ZKVM_U256_INLINE_H

#define ZKVM_U256_I static inline __attribute__((always_inline))

/* ---- helpers --------------------------------------------------------------- */

ZKVM_U256_I uint64_t zkvm_u256_i_bswap(uint64_t x) {
#if defined(__riscv_zbb)
    return __builtin_bswap64(x);   /* rev8 */
#else
    x = ((x & 0x00ff00ff00ff00ffull) << 8) | ((x >> 8) & 0x00ff00ff00ff00ffull);
    x = ((x & 0x0000ffff0000ffffull) << 16) | ((x >> 16) & 0x0000ffff0000ffffull);
    return (x << 32) | (x >> 32);
#endif
}

/* Big-endian bytes <-> little-endian limbs (limb 0 = least significant). */
ZKVM_U256_I void zkvm_u256_i_ld(const zkvm_u256* p, uint64_t l[4]) {
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) {
        uint64_t w;
        __builtin_memcpy(&w, p->data + 8 * (3 - i), 8);
        l[i] = zkvm_u256_i_bswap(w);
    }
}
ZKVM_U256_I void zkvm_u256_i_st(zkvm_u256* p, const uint64_t l[4]) {
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) {
        uint64_t w = zkvm_u256_i_bswap(l[i]);
        __builtin_memcpy(p->data + 8 * (3 - i), &w, 8);
    }
}
ZKVM_U256_I void zkvm_u256_i_st_small(zkvm_u256* p, uint64_t v) {
    uint64_t l[4] = {v, 0, 0, 0};
    zkvm_u256_i_st(p, l);
}
/* eq/iszero/and/or/xor/not don't care about byte order: they use the raw words. */
ZKVM_U256_I void zkvm_u256_i_ldw(const zkvm_u256* p, uint64_t w[4]) {
    __builtin_memcpy(w, p->data, 32);
}
ZKVM_U256_I void zkvm_u256_i_stw(zkvm_u256* p, const uint64_t w[4]) {
    __builtin_memcpy(p->data, w, 32);
}
ZKVM_U256_I int zkvm_u256_i_is_zero(const uint64_t l[4]) {
    return (l[0] | l[1] | l[2] | l[3]) == 0;
}
/* A shift/index/byte operand below `limit`, as a plain number; `limit` if not. */
ZKVM_U256_I uint64_t zkvm_u256_i_small(const uint64_t l[4], uint64_t limit) {
    return (l[1] | l[2] | l[3]) == 0 && l[0] < limit ? l[0] : limit;
}
ZKVM_U256_I int zkvm_u256_i_ult(const uint64_t a[4], const uint64_t b[4]) {
    _Pragma("GCC unroll 4")
    for (int i = 3; i >= 0; i--)
        if (a[i] != b[i]) return a[i] < b[i];
    return 0;
}
ZKVM_U256_I int zkvm_u256_i_slt(const uint64_t a[4], const uint64_t b[4]) {
    const uint64_t s = 1ull << 63;
    uint64_t x[4] = {a[0], a[1], a[2], a[3] ^ s}, y[4] = {b[0], b[1], b[2], b[3] ^ s};
    return zkvm_u256_i_ult(x, y);
}
/* arith256: a * b + c = dh:dl.  arith256_mod: d = (a * b + c) mod m, m != 0. */
ZKVM_U256_I void zkvm_u256_i_arith256(const uint64_t a[4], const uint64_t b[4],
                                      const uint64_t c[4], uint64_t dl[4], uint64_t dh[4]) {
    const void* p[5] = {a, b, c, dl, dh};
    __asm__ volatile("csrs 0x801, %0" : : "r"(p) : "memory");
}
ZKVM_U256_I void zkvm_u256_i_arith256_mod(const uint64_t a[4], const uint64_t b[4],
                                          const uint64_t c[4], const uint64_t m[4],
                                          uint64_t d[4]) {
    const void* p[5] = {a, b, c, m, d};
    __asm__ volatile("csrs 0x802, %0" : : "r"(p) : "memory");
}
/* z = x >> s for s < 256, shifting in `fill` (0, or all ones for sar). */
ZKVM_U256_I void zkvm_u256_i_shr(const uint64_t x[4], unsigned s, uint64_t fill,
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

ZKVM_U256_I zkvm_status zkvm_u256_add(const zkvm_u256* a, const zkvm_u256* b,
                                      zkvm_u256* result) {
    uint64_t x[4], y[4], z[4], c = 0;
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y);
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) {
        uint64_t s = x[i] + c;
        uint64_t c1 = s < c;
        z[i] = s + y[i];
        c = c1 | (z[i] < s);
    }
    zkvm_u256_i_st(result, z);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_sub(const zkvm_u256* a, const zkvm_u256* b,
                                      zkvm_u256* result) {
    uint64_t x[4], y[4], z[4], w = 0;
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y);
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) {
        uint64_t d = x[i] - y[i];
        uint64_t w1 = x[i] < y[i];
        z[i] = d - w;
        w = w1 | (d < w);
    }
    zkvm_u256_i_st(result, z);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_mul(const zkvm_u256* a, const zkvm_u256* b,
                                      zkvm_u256* result) {
    uint64_t x[4], y[4], zero[4] = {0, 0, 0, 0}, lo[4], hi[4];
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y);
    zkvm_u256_i_arith256(x, y, zero, lo, hi);
    zkvm_u256_i_st(result, lo);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_addmod(const zkvm_u256* a, const zkvm_u256* b,
                                         const zkvm_u256* n, zkvm_u256* result) {
    uint64_t x[4], y[4], m[4], one[4] = {1, 0, 0, 0}, d[4] = {0, 0, 0, 0};
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y); zkvm_u256_i_ld(n, m);
    if (!zkvm_u256_i_is_zero(m)) zkvm_u256_i_arith256_mod(x, one, y, m, d);  /* a*1 + b */
    zkvm_u256_i_st(result, d);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_mulmod(const zkvm_u256* a, const zkvm_u256* b,
                                         const zkvm_u256* n, zkvm_u256* result) {
    uint64_t x[4], y[4], m[4], zero[4] = {0, 0, 0, 0}, d[4] = {0, 0, 0, 0};
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y); zkvm_u256_i_ld(n, m);
    if (!zkvm_u256_i_is_zero(m)) zkvm_u256_i_arith256_mod(x, y, zero, m, d);  /* a*b + 0 */
    zkvm_u256_i_st(result, d);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_exp(const zkvm_u256* base, const zkvm_u256* exponent,
                                      zkvm_u256* result) {
    uint64_t b[4], e[4], acc[4] = {1, 0, 0, 0}, zero[4] = {0, 0, 0, 0}, hi[4];
    zkvm_u256_i_ld(base, b); zkvm_u256_i_ld(exponent, e);
    int top = 255;                                     /* skip the exponent's leading zeros */
    while (top >= 0 && !((e[top / 64] >> (top % 64)) & 1)) top--;
    for (int i = top; i >= 0; i--) {                   /* left-to-right square-and-multiply */
        zkvm_u256_i_arith256(acc, acc, zero, acc, hi);
        if ((e[i / 64] >> (i % 64)) & 1) zkvm_u256_i_arith256(acc, b, zero, acc, hi);
    }
    zkvm_u256_i_st(result, acc);
    return ZKVM_EOK;
}

/* ---- comparisons ----------------------------------------------------------- */

ZKVM_U256_I zkvm_status zkvm_u256_lt(const zkvm_u256* a, const zkvm_u256* b,
                                     zkvm_u256* result) {
    uint64_t x[4], y[4];
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y);
    zkvm_u256_i_st_small(result, zkvm_u256_i_ult(x, y));
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_gt(const zkvm_u256* a, const zkvm_u256* b,
                                     zkvm_u256* result) {
    uint64_t x[4], y[4];
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y);
    zkvm_u256_i_st_small(result, zkvm_u256_i_ult(y, x));
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_slt(const zkvm_u256* a, const zkvm_u256* b,
                                      zkvm_u256* result) {
    uint64_t x[4], y[4];
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y);
    zkvm_u256_i_st_small(result, zkvm_u256_i_slt(x, y));
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_sgt(const zkvm_u256* a, const zkvm_u256* b,
                                      zkvm_u256* result) {
    uint64_t x[4], y[4];
    zkvm_u256_i_ld(a, x); zkvm_u256_i_ld(b, y);
    zkvm_u256_i_st_small(result, zkvm_u256_i_slt(y, x));
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_eq(const zkvm_u256* a, const zkvm_u256* b,
                                     zkvm_u256* result) {
    uint64_t x[4], y[4];
    zkvm_u256_i_ldw(a, x); zkvm_u256_i_ldw(b, y);
    zkvm_u256_i_st_small(result,
                         ((x[0] ^ y[0]) | (x[1] ^ y[1]) | (x[2] ^ y[2]) | (x[3] ^ y[3])) == 0);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_iszero(const zkvm_u256* a, zkvm_u256* result) {
    uint64_t x[4];
    zkvm_u256_i_ldw(a, x);
    zkvm_u256_i_st_small(result, zkvm_u256_i_is_zero(x));
    return ZKVM_EOK;
}

/* ---- bitwise --------------------------------------------------------------- */

#define ZKVM_U256_I_BITWISE(name, op)                                                 \
    ZKVM_U256_I zkvm_status zkvm_u256_##name(const zkvm_u256* a, const zkvm_u256* b,  \
                                             zkvm_u256* result) {                     \
        uint64_t x[4], y[4], z[4];                                                    \
        zkvm_u256_i_ldw(a, x); zkvm_u256_i_ldw(b, y);                                 \
        _Pragma("GCC unroll 4")                                                       \
        for (int i = 0; i < 4; i++) z[i] = x[i] op y[i];                              \
        zkvm_u256_i_stw(result, z);                                                   \
        return ZKVM_EOK;                                                              \
    }
ZKVM_U256_I_BITWISE(and, &)
ZKVM_U256_I_BITWISE(or, |)
ZKVM_U256_I_BITWISE(xor, ^)
#undef ZKVM_U256_I_BITWISE

ZKVM_U256_I zkvm_status zkvm_u256_not(const zkvm_u256* a, zkvm_u256* result) {
    uint64_t x[4];
    zkvm_u256_i_ldw(a, x);
    _Pragma("GCC unroll 4")
    for (int i = 0; i < 4; i++) x[i] = ~x[i];
    zkvm_u256_i_stw(result, x);
    return ZKVM_EOK;
}
/* byte(i, a): the i-th big-endian byte, which is exactly a->data[i]. */
ZKVM_U256_I zkvm_status zkvm_u256_byte(const zkvm_u256* i, const zkvm_u256* a,
                                       zkvm_u256* result) {
    uint64_t n[4];
    zkvm_u256_i_ld(i, n);
    uint64_t k = zkvm_u256_i_small(n, 32);
    zkvm_u256_i_st_small(result, k < 32 ? a->data[k] : 0);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_shl(const zkvm_u256* shift, const zkvm_u256* value,
                                      zkvm_u256* result) {
    uint64_t n[4], x[4], z[4] = {0, 0, 0, 0};
    zkvm_u256_i_ld(shift, n); zkvm_u256_i_ld(value, x);
    uint64_t s = zkvm_u256_i_small(n, 256);
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
    zkvm_u256_i_st(result, z);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_shr(const zkvm_u256* shift, const zkvm_u256* value,
                                      zkvm_u256* result) {
    uint64_t n[4], x[4], z[4] = {0, 0, 0, 0};
    zkvm_u256_i_ld(shift, n); zkvm_u256_i_ld(value, x);
    uint64_t s = zkvm_u256_i_small(n, 256);
    if (s < 256) zkvm_u256_i_shr(x, (unsigned)s, 0, z);
    zkvm_u256_i_st(result, z);
    return ZKVM_EOK;
}
ZKVM_U256_I zkvm_status zkvm_u256_sar(const zkvm_u256* shift, const zkvm_u256* value,
                                      zkvm_u256* result) {
    uint64_t n[4], x[4], z[4];
    zkvm_u256_i_ld(shift, n); zkvm_u256_i_ld(value, x);
    uint64_t fill = (int64_t)x[3] < 0 ? ~0ull : 0;
    uint64_t s = zkvm_u256_i_small(n, 256);
    if (s < 256) zkvm_u256_i_shr(x, (unsigned)s, fill, z);
    else z[0] = z[1] = z[2] = z[3] = fill;
    zkvm_u256_i_st(result, z);
    return ZKVM_EOK;
}

/* ---- extended -------------------------------------------------------------- */

/* signextend(b, value): extend the sign of byte b (0 = least significant). */
ZKVM_U256_I zkvm_status zkvm_u256_signextend(const zkvm_u256* b, const zkvm_u256* value,
                                             zkvm_u256* result) {
    uint64_t n[4], x[4];
    zkvm_u256_i_ld(b, n); zkvm_u256_i_ld(value, x);
    uint64_t k = zkvm_u256_i_small(n, 31);
    if (k < 31) {
        unsigned bit = 8 * (unsigned)k + 7, q = bit / 64, s = bit % 64;
        uint64_t fill = (x[q] >> s) & 1 ? ~0ull : 0;
        uint64_t keep = s == 63 ? ~0ull : (2ull << s) - 1;
        x[q] = (x[q] & keep) | (fill & ~keep);
        for (unsigned i = q + 1; i < 4; i++) x[i] = fill;
    }
    zkvm_u256_i_st(result, x);
    return ZKVM_EOK;
}

#undef ZKVM_U256_I

#endif /* ZKVM_U256_INLINE_H */
