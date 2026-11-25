// dot_bf16 - experimental exact BF16 dot product, rounded once to f32 (see
// core/src/operations/dot_bf16.rs and zisk_definitions::dot_bf16, the reference this code must
// match bit for bit).
//
// Called through the wrappers in dot_bf16.asm, which save the registers the generated code keeps
// values in. The parameter block p holds six qwords: x address, y address, n, flags (0), block
// (0 = n) and out (0 = no block results). The Rust emulator checks the parameters; this code
// trusts them.
//
// The sum is kept exactly, with the least significant bit 2^-266 (the product of two smallest
// BF16 subnormals): in carry-save digits while adding, then as a 576-bit two's complement value,
// which is rounded once to f32, to nearest, ties to even. A zero sum gives +0.0; any infinite or NaN input gives the canonical NaN.

#include <stdint.h>
#include <string.h>

#include "dot_bf16.h"

#define WORDS 9
#define LSB_EXP (-266)
#define F32_MIN_SUBNORMAL_BIT (-149 - LSB_EXP)
#define CANONICAL_NAN 0x7FC00000u

typedef struct
{
    uint64_t w[WORDS];
} acc_t;

// The accumulator while adding products: 18 signed 32-bit digits, each kept in an int64_t
// without propagating carries (carry-save). A product (below 2^16, shifted by up to 31 bits)
// adds less than 2^32 to each of two digits, so a digit stays below 2^52 in absolute value for
// the at most 2^20 products of a call: exact, with no carry chain per product.
#define LIMBS (2 * WORDS)

typedef struct
{
    int64_t d[LIMBS];
} lazy_t;

// Adds (or subtracts, if neg) sig (below 2^30) at bit position pos (0..512)
static inline void lazy_add(lazy_t *a, int neg, uint64_t sig, unsigned pos)
{
    unsigned i = pos / 32, off = pos % 32;
    uint64_t v = sig << off;                   // below 2^61
    int64_t lo = (int64_t)(v & 0xFFFFFFFFu), hi = (int64_t)(v >> 32);
    // Branch-free sign (the signs of real data are random): m is 0 or -1, and (v ^ m) - m is v
    // or -v
    int64_t m = -(int64_t)neg;
    a->d[i] += (lo ^ m) - m;
    a->d[i + 1] += (hi ^ m) - m;
}

// Propagates the carries of a lazy accumulator into a 576-bit two's complement value
static inline void lazy_normalize(const lazy_t *a, acc_t *out)
{
    int64_t carry = 0;
    uint32_t digits[LIMBS];
    for (unsigned k = 0; k < LIMBS; k++)
    {
        int64_t v = a->d[k] + carry;
        digits[k] = (uint32_t)v;
        carry = v >> 32;                       // arithmetic shift: floor division by 2^32
    }
    // The sum fits in 576 bits, so the remaining carry is the sign extension (0 or -1)
    for (unsigned k = 0; k < WORDS; k++)
        out->w[k] = (uint64_t)digits[2 * k] | ((uint64_t)digits[2 * k + 1] << 32);
}

static inline int bit_at(const uint64_t *m, int pos)
{
    return (m[pos / 64] >> (pos % 64)) & 1;
}

// count (at most 25) bits of m from bit pos
static inline uint64_t bits_at(const uint64_t *m, int pos, int count)
{
    int i = pos / 64, off = pos % 64;
    uint64_t lo = m[i] >> off;
    uint64_t hi = (off && i + 1 < WORDS) ? m[i + 1] << (64 - off) : 0;
    return (lo | hi) & ((1ull << count) - 1);
}

// Whether any bit of m below bit pos is set
static inline int any_below(const uint64_t *m, int pos)
{
    int i = pos / 64, off = pos % 64;
    for (int k = 0; k < i; k++)
        if (m[k]) return 1;
    return off && (m[i] & ((1ull << off) - 1));
}

// The sum rounded to f32, to nearest, ties to even, as f32 bits; +0.0 for a zero sum
static uint32_t acc_to_f32(const acc_t *a)
{
    int neg = (int)(a->w[WORDS - 1] >> 63);
    uint64_t m[WORDS];
    if (neg)
    {
        uint64_t carry = 1;
        for (unsigned k = 0; k < WORDS; k++)
        {
            m[k] = ~a->w[k] + carry;
            carry = carry && m[k] == 0;
        }
    }
    else
    {
        memcpy(m, a->w, sizeof(m));
    }
    int top = -1;
    for (int k = WORDS - 1; k >= 0; k--)
    {
        if (m[k])
        {
            top = k * 64 + 63 - __builtin_clzll(m[k]);
            break;
        }
    }
    if (top < 0) return 0;

    int lsb = top - 23 > F32_MIN_SUBNORMAL_BIT ? top - 23 : F32_MIN_SUBNORMAL_BIT;
    uint64_t q = bits_at(m, lsb, top - lsb + 1);
    int half = lsb > 0 && bit_at(m, lsb - 1);
    int sticky = lsb > 1 && any_below(m, lsb - 1);
    if (half && (sticky || (q & 1)))
    {
        q += 1;
        if (q == (1ull << 24))
        {
            q = 1ull << 23;
            lsb += 1;
        }
    }
    uint32_t sign = (uint32_t)neg << 31;
    if (q < (1ull << 23)) return sign | (uint32_t)q;  // subnormal: lsb is 2^-149
    int biased = (lsb + LSB_EXP) + 23 + 127;
    if (biased >= 0xff) return sign | 0x7F800000u;
    return sign | ((uint32_t)biased << 23) | ((uint32_t)q & 0x7FFFFF);
}

// Adds x[start..end) * y[start..end) to a; returns 1 if a value was infinite or NaN
static inline int block_sum(lazy_t *a, const uint16_t *x, const uint16_t *y, uint64_t start,
                            uint64_t end)
{
    int special = 0;
    for (uint64_t i = start; i < end; i++)
    {
        uint16_t bx = x[i], by = y[i];
        unsigned ex = (bx >> 7) & 0xff, ey = (by >> 7) & 0xff;
        special |= ex == 0xff || ey == 0xff;
        uint32_t sx = (bx & 0x7f) | (ex ? 0x80 : 0), sy = (by & 0x7f) | (ey ? 0x80 : 0);
        // An infinite or NaN operand makes the whole result NaN, so its product is skipped
        uint64_t sig = (ex == 0xff || ey == 0xff) ? 0 : (uint64_t)sx * sy;
        // value = sig * 2^(ex' + ey' - 268) with e' = max(e, 1); the accumulator's bit 0 is
        // 2^-266, so pos = ex' + ey' - 2 (at most 506 for finite values)
        unsigned pos = (ex ? ex : 1) + (ey ? ey : 1) - 2;
        if (pos > 506) pos = 0;                // only for skipped products (sig = 0)
        lazy_add(a, ((bx ^ by) >> 15) & 1, sig, pos);
    }
    return special;
}

// ----- FP4 weights (flags 1 = MXFP4, 2 = NVFP4) with BF16 values -----

#define FLAGS_MXFP4 1
#define FLAGS_NVFP4 2

// E2M1 values in units of 2^-1, signed
static const int32_t E2M1_UNITS[16] = { 0, 1, 2, 3, 4, 6, 8, 12, 0, -1, -2, -3, -4, -6, -8, -12 };

// E4M3 (OCP FP8) values in units of 2^-9, signed; NaN codes (0x7F, 0xFF) are 0 here and are
// detected separately
#define E43_EXP(c) (((c) >> 3) & 15)
#define E43_MAN(c) ((c) & 7)
#define E43_MAG(c) (E43_EXP(c) == 0 ? E43_MAN(c) : (8 + E43_MAN(c)) << (E43_EXP(c) - 1))
#define E43(c) ((((c) & 0x7f) == 0x7f) ? 0 : (((c) & 0x80) ? -(E43_MAG(c)) : E43_MAG(c)))
#define Q4(F, b) F(b), F((b) + 1), F((b) + 2), F((b) + 3)
#define Q16(F, b) Q4(F, b), Q4(F, (b) + 4), Q4(F, (b) + 8), Q4(F, (b) + 12)
#define Q64(F, b) Q16(F, b), Q16(F, (b) + 16), Q16(F, (b) + 32), Q16(F, (b) + 48)
static const int32_t E4M3_UNITS[256] = { Q64(E43, 0), Q64(E43, 64), Q64(E43, 128), Q64(E43, 192) };

// Adds w[start..end) * y[start..end) to a, with w[i] = e2m1(x[i]) * scale[i / fb]; returns 1
// if a BF16 value was infinite or NaN, or a scale NaN
static inline int fp4_block_sum(lazy_t *a, int flags, const uint8_t *x, const uint8_t *scales,
                                const uint16_t *y, uint64_t start, uint64_t end)
{
    unsigned fb = flags == FLAGS_MXFP4 ? 32 : 16;
    int special = 0;
    for (uint64_t i = start; i < end; i++)
    {
        uint8_t b = x[i >> 1];
        int32_t u = E2M1_UNITS[(i & 1) ? (b >> 4) : (b & 15)];
        uint8_t s = scales[i / fb];
        uint16_t by = y[i];
        unsigned ey = (by >> 7) & 0xff;
        uint32_t sy = (by & 0x7f) | (ey ? 0x80 : 0);
        int neg = (u < 0) ^ ((by >> 15) & 1);
        uint64_t wsig;
        unsigned pos;
        int bad;
        if (flags == FLAGS_MXFP4)
        {
            // w = u * 2^(s - 128), y = sy * 2^(ey' - 134), bit 0 is 2^-266: pos = s + ey' + 4
            bad = s == 0xff;
            wsig = (uint64_t)(u < 0 ? -u : u);
            pos = s + (ey ? ey : 1) + 4;
        }
        else
        {
            // w = u * e * 2^-10 with e the E4M3 scale in units of 2^-9: pos = ey' + 122
            int32_t e = E4M3_UNITS[s];
            bad = (s & 0x7f) == 0x7f;
            neg ^= e < 0;
            wsig = (uint64_t)(u < 0 ? -u : u) * (uint64_t)(e < 0 ? -e : e);
            pos = (ey ? ey : 1) + 122;
        }
        bad |= ey == 0xff;
        special |= bad;
        uint64_t sig = bad ? 0 : wsig * sy;   // below 2^30
        if (bad) pos = 0;
        lazy_add(a, neg, sig, pos);
    }
    return special;
}

static uint32_t fp4_compute(const uint64_t *p)
{
    const uint8_t *x = (const uint8_t *)p[0];
    const uint16_t *y = (const uint16_t *)p[1];
    uint64_t n = p[2];
    int flags = (int)p[3];
    uint64_t block = p[4] ? p[4] : n;
    uint32_t *out = (uint32_t *)p[5];
    const uint8_t *scales = (const uint8_t *)p[6];
    if (n == 0) return 0;

    lazy_t total;
    memset(&total, 0, sizeof(total));
    int total_special = 0;
    for (uint64_t b = 0, start = 0; start < n; b++, start += block)
    {
        lazy_t a;
        memset(&a, 0, sizeof(a));
        int special = fp4_block_sum(&a, flags, x, scales, y, start, start + block);
        if (out)
        {
            acc_t r;
            lazy_normalize(&a, &r);
            out[b] = special ? CANONICAL_NAN : acc_to_f32(&r);
        }
        for (unsigned k = 0; k < LIMBS; k++) total.d[k] += a.d[k];
        total_special |= special;
    }
    if (total_special) return CANONICAL_NAN;
    acc_t r;
    lazy_normalize(&total, &r);
    return acc_to_f32(&r);
}

uint32_t dot_bf16_compute(const uint64_t *p)
{
    if (p[3] != 0) return fp4_compute(p);
    const uint16_t *x = (const uint16_t *)p[0];
    const uint16_t *y = (const uint16_t *)p[1];
    uint64_t n = p[2];
    uint64_t block = p[4] ? p[4] : n;
    uint32_t *out = (uint32_t *)p[5];
    if (n == 0) return 0;

    lazy_t total;
    memset(&total, 0, sizeof(total));
    int total_special = 0;
    for (uint64_t b = 0, start = 0; start < n; b++, start += block)
    {
        lazy_t a;
        memset(&a, 0, sizeof(a));
        int special = block_sum(&a, x, y, start, start + block);
        if (out)
        {
            acc_t r;
            lazy_normalize(&a, &r);
            out[b] = special ? CANONICAL_NAN : acc_to_f32(&r);
        }
        for (unsigned k = 0; k < LIMBS; k++) total.d[k] += a.d[k];
        total_special |= special;
    }
    if (total_special) return CANONICAL_NAN;
    acc_t r;
    lazy_normalize(&total, &r);
    return acc_to_f32(&r);
}

dot_bf16_mtrace_ret dot_bf16_mtrace_c(const uint64_t *p, uint64_t *trace)
{
    // Header: the first 6 words of the parameter block and the result, then the data as words:
    // flags 0:    x, y
    // flags 1, 2: the scales address, x (n/16 words), the scales (rounded up to words), y
    uint64_t n = p[2], y_words = n / 4;
    for (int i = 0; i < 6; i++) trace[i] = p[i];
    uint64_t *t = trace + 7;
    const uint64_t *x = (const uint64_t *)p[0], *y = (const uint64_t *)p[1];
    if (p[3] == 0)
    {
        for (uint64_t i = 0; i < y_words; i++) *t++ = x[i];
    }
    else
    {
        uint64_t fb = p[3] == FLAGS_MXFP4 ? 32 : 16;
        const uint64_t *s = (const uint64_t *)p[6];
        *t++ = p[6];
        for (uint64_t i = 0; i < n / 16; i++) *t++ = x[i];
        for (uint64_t i = 0; i < (n / fb + 7) / 8; i++) *t++ = s[i];
    }
    for (uint64_t i = 0; i < y_words; i++) *t++ = y[i];
    uint32_t result = dot_bf16_compute(p);
    trace[6] = result;
    dot_bf16_mtrace_ret r = { result, (uint64_t)(t - trace) };
    return r;
}
