// dot_f8 - experimental exact dot product of int8, FP8 and FP4 vectors (see
// core/src/operations/dot_f8.rs and zisk_definitions::dot_f8 for the formats).
//
// Called through the wrappers in dot_f8.asm, which save the registers the generated code keeps
// values in. The parameter block p holds six qwords: x address, y address, n, formats (bits 0..8
// format of x, 8..16 format of y, bit 16 REPEAT_Y: y is one block, read again for each block),
// block (0 = n) and out (0 = no block sums). The Rust emulator checks the parameters; this code
// trusts them.

#include <stdint.h>
#include <emmintrin.h>

#include "dot_f8.h"

// Format IDs, as in zisk_definitions::dot_f8
#define F_I8 0
#define F_E4M3 1
#define F_E4M3_FNUZ 2
#define F_E2M1 3
#define REPEAT_Y (1ULL << 16)

// Decoding tables, in each format's units (2^0, 2^-9, 2^-10 and 2^-1); NaN codes are 0. The E2M1
// table is indexed by nibble.
#define SGN(c, mag) (((c) & 0x80) ? -(mag) : (mag))
#define EXP(c) (((c) >> 3) & 15)
#define MAN(c) ((c) & 7)
#define MAG43(c) (EXP(c) == 0 ? MAN(c) : (8 + MAN(c)) << (EXP(c) - 1))
#define D_I8(c) ((c) < 128 ? (c) : (c) - 256)
#define D_E4M3(c) ((((c) & 0x7f) == 0x7f) ? 0 : SGN(c, MAG43(c)))
#define D_E4M3_FNUZ(c) (((c) == 0x80) ? 0 : SGN(c, MAG43(c)))
#define E21(c) ((((c) & 15) >> 1) & 3)
#define M21(c) ((c) & 1)
#define D_E2M1(c) ((((c) & 8) ? -1 : 1) * (E21(c) == 0 ? M21(c) : (2 + M21(c)) << (E21(c) - 1)))

#define R4(F, b) F(b), F((b) + 1), F((b) + 2), F((b) + 3)
#define R16(F, b) R4(F, b), R4(F, (b) + 4), R4(F, (b) + 8), R4(F, (b) + 12)
#define R64(F, b) R16(F, b), R16(F, (b) + 16), R16(F, (b) + 32), R16(F, (b) + 48)
#define R256(F) R64(F, 0), R64(F, 64), R64(F, 128), R64(F, 192)

static const int32_t dot_f8_tables[4][256] = {
    { R256(D_I8) },
    { R256(D_E4M3) },
    { R256(D_E4M3_FNUZ) },
    { R256(D_E2M1) },
};

// Value i of a vector, 8 or 4 bits per value (low nibble first)
static inline __attribute__((always_inline)) int64_t value(const uint8_t *v, const int32_t *t,
                                                            int is4, uint64_t i)
{
    if (is4)
    {
        uint8_t b = v[i >> 1];
        return t[(i & 1) ? (b >> 4) : (b & 15)];
    }
    return t[v[i]];
}

// Sum of x[start + i]*y[ystart + i] for i in [0, len), with the bit widths known at compile time
static inline __attribute__((always_inline)) int64_t block_sum(
    const uint8_t *x, const int32_t *tx, int x4,
    const uint8_t *y, const int32_t *ty, int y4,
    uint64_t start, uint64_t ystart, uint64_t len)
{
    int64_t acc = 0;
    for (uint64_t i = 0; i < len; i++)
    {
        acc += value(x, tx, x4, start + i) * value(y, ty, y4, ystart + i);
    }
    return acc;
}

// int8 x int8 with SSE2: 8 values per step, i32 lanes flushed to i64 every 4096 steps (each lane
// gains at most 2 * 128 * 128 = 2^15 per step)
static int64_t block_sum_i8(const uint8_t *x, const uint8_t *y, uint64_t start, uint64_t ystart,
                            uint64_t len)
{
    int64_t acc = 0;
    uint64_t words = len / 8;
    const uint8_t *px = x + start, *py = y + ystart;
    while (words > 0)
    {
        uint64_t chunk = words < 4096 ? words : 4096;
        __m128i lanes = _mm_setzero_si128();
        for (uint64_t w = 0; w < chunk; w++)
        {
            __m128i a = _mm_loadl_epi64((const __m128i *)(px + 8 * w));
            __m128i b = _mm_loadl_epi64((const __m128i *)(py + 8 * w));
            a = _mm_srai_epi16(_mm_unpacklo_epi8(a, a), 8);
            b = _mm_srai_epi16(_mm_unpacklo_epi8(b, b), 8);
            lanes = _mm_add_epi32(lanes, _mm_madd_epi16(a, b));
        }
        int32_t l[4];
        _mm_storeu_si128((__m128i *)l, lanes);
        acc += (int64_t)l[0] + l[1] + l[2] + l[3];
        px += 8 * chunk;
        py += 8 * chunk;
        words -= chunk;
    }
    return acc;
}

int64_t dot_f8_compute(const uint64_t *p)
{
    const uint8_t *x = (const uint8_t *)p[0];
    const uint8_t *y = (const uint8_t *)p[1];
    uint64_t n = p[2];
    uint32_t fx = p[3] & 0xff, fy = (p[3] >> 8) & 0xff;
    uint64_t block = p[4] ? p[4] : n;
    int64_t *out = (int64_t *)p[5];
    if (n == 0) return 0;

    const int32_t *tx = dot_f8_tables[fx], *ty = dot_f8_tables[fy];
    int x4 = fx == F_E2M1, y4 = fy == F_E2M1;
    int64_t total = 0;
    int repeat_y = (p[3] & REPEAT_Y) != 0;
    for (uint64_t b = 0, start = 0; start < n; b++, start += block)
    {
        int64_t s;
        uint64_t ys = repeat_y ? 0 : start;
        if (fx == F_I8 && fy == F_I8) s = block_sum_i8(x, y, start, ys, block);
        else if (!x4 && !y4) s = block_sum(x, tx, 0, y, ty, 0, start, ys, block);
        else if (x4 && !y4) s = block_sum(x, tx, 1, y, ty, 0, start, ys, block);
        else if (!x4 && y4) s = block_sum(x, tx, 0, y, ty, 1, start, ys, block);
        else s = block_sum(x, tx, 1, y, ty, 1, start, ys, block);
        if (out) out[b] = s;
        total += s;
    }
    return total;
}

dot_f8_mtrace_ret dot_f8_mtrace_c(const uint64_t *p, uint64_t *trace)
{
    // Header: the parameter block and the result, then both vectors as words (y one block with
    // REPEAT_Y)
    uint64_t n = p[2];
    uint64_t yn = (p[3] & REPEAT_Y) && p[4] < n ? p[4] : n;
    uint64_t xw = (p[3] & 0xff) == F_E2M1 ? n / 16 : n / 8;
    uint64_t yw = ((p[3] >> 8) & 0xff) == F_E2M1 ? yn / 16 : yn / 8;
    for (int i = 0; i < 6; i++) trace[i] = p[i];
    const uint64_t *x = (const uint64_t *)p[0], *y = (const uint64_t *)p[1];
    uint64_t *t = trace + 7;
    for (uint64_t i = 0; i < xw; i++) *t++ = x[i];
    for (uint64_t i = 0; i < yw; i++) *t++ = y[i];
    int64_t total = dot_f8_compute(p);
    trace[6] = (uint64_t)total;
    dot_f8_mtrace_ret r = { total, 7 + xw + yw };
    return r;
}
