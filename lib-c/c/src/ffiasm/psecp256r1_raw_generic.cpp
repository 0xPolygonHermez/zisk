#include "psecp256r1_element.hpp"
#include <gmp.h>
#include <cstring>

// uint64_t and mp_limb_t are distinct types on some platforms (e.g. macOS)
static_assert(sizeof(mp_limb_t) == sizeof(uint64_t), "64-bit GMP limbs required");
static inline mp_ptr    L(uint64_t *p)       { return reinterpret_cast<mp_ptr>(p); }
static inline mp_srcptr L(const uint64_t *p) { return reinterpret_cast<mp_srcptr>(p); }

static uint64_t     pSecp256r1_rawq[] = {0xffffffffffffffff,0x00000000ffffffff,0x0000000000000000,0xffffffff00000001, 0};
static pSecp256r1RawElement pSecp256r1_rawR2  = {0x0000000000000003,0xfffffffbffffffff,0xfffffffffffffffe,0x00000004fffffffd};
static uint64_t     pSecp256r1_np     = 0x1;
static uint64_t     lboMask   = 0xffffffffffffffff;
static pSecp256r1RawElement zero      = {0};


void pSecp256r1_rawAdd(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB)
{
    uint64_t carry = mpn_add_n(L(pRawResult), L(pRawA), L(pRawB), pSecp256r1_N64);

    if(carry || mpn_cmp(L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawAddLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_add_1(L(pRawResult), L(pRawA), pSecp256r1_N64, rawB);

    if(carry || mpn_cmp(L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawSub(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB)
{
    uint64_t carry = mpn_sub_n(L(pRawResult), L(pRawA), L(pRawB), pSecp256r1_N64);

    if(carry)
    {
        mpn_add_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawSubRegular(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    mpn_sub_n(L(pRawResult), L(pRawA), L(pRawB), pSecp256r1_N64);
}

void pSecp256r1_rawSubSL(pSecp256r1RawElement pRawResult, uint64_t rawA, pSecp256r1RawElement pRawB)
{
    pSecp256r1RawElement pRawA = {rawA};

    uint64_t carry = mpn_sub_n(L(pRawResult), L(pRawA), L(pRawB), pSecp256r1_N64);

    if(carry)
    {
        mpn_add_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawSubLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_sub_1(L(pRawResult), L(pRawA), pSecp256r1_N64, rawB);

    if(carry)
    {
        mpn_add_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawNeg(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA)
{
    if (mpn_cmp(L(pRawA), L(zero), pSecp256r1_N64) != 0)
    {
        mpn_sub_n(L(pRawResult), L(pSecp256r1_rawq), L(pRawA), pSecp256r1_N64);
    }
    else
    {
        mpn_copyi(L(pRawResult), L(zero), pSecp256r1_N64);
    }
}

//  Substracts a long element and a short element form 0
void pSecp256r1_rawNegLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB)
{
    uint64_t carry1 = mpn_sub_1(L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64, rawB);
    uint64_t carry2 = mpn_sub_n(L(pRawResult), L(pRawResult), L(pRawA), pSecp256r1_N64);

    if (carry1 || carry2)
    {
        mpn_add_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawCopy(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA)
{
    memcpy(pRawResult, pRawA, sizeof(pSecp256r1RawElement));
}

int pSecp256r1_rawIsEq(const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB)
{
    return mpn_cmp(L(pRawA), L(pRawB), pSecp256r1_N64) == 0;
}

void pSecp256r1_rawMMul(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB)
{
    const mp_size_t  N = pSecp256r1_N64+1;
    const uint64_t  *mq = pSecp256r1_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(L(product0), L(pRawB), pSecp256r1_N64, pRawA[0]);

    np0 = pSecp256r1_np * product0[0];
    product1[N-1] += mpn_addmul_1(L(product0), L(mq), N, np0);

    product1[N-1] += mpn_addmul_1(L(product1), L(pRawB), pSecp256r1_N64, pRawA[1]);
    product2[N-1] = mpn_add(L(product1), L(product1), N, L(product0+1), N-1);

    np0 = pSecp256r1_np * product1[0];
    product2[N-1] += mpn_addmul_1(L(product1), L(mq), N, np0);

    product2[N-1] += mpn_addmul_1(L(product2), L(pRawB), pSecp256r1_N64, pRawA[2]);
    product3[N-1] = mpn_add(L(product2), L(product2), N, L(product1+1), N-1);

    np0 = pSecp256r1_np * product2[0];
    product3[N-1] += mpn_addmul_1(L(product2), L(mq), N, np0);

    product3[N-1] += mpn_addmul_1(L(product3), L(pRawB), pSecp256r1_N64, pRawA[3]);
    c = mpn_add(L(product3), L(product3), N, L(product2+1), N-1);

    np0 = pSecp256r1_np * product3[0];
    c += mpn_addmul_1(L(product3), L(mq), N, np0);

    mpn_copyi(L(pRawResult), L(product3+1), pSecp256r1_N64);

    if (c || mpn_cmp(L(pRawResult), L(mq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(mq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawMSquare(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA)
{
    pSecp256r1_rawMMul(pRawResult, pRawA, pRawA);
}

void pSecp256r1_rawMMul1(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, uint64_t pRawB)
{
    const mp_size_t  N = pSecp256r1_N64+1;
    const uint64_t  *mq = pSecp256r1_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(L(product0), L(pRawA), pSecp256r1_N64, pRawB);

    np0 = pSecp256r1_np * product0[0];
    product1[N-1] = mpn_addmul_1(L(product0), L(mq), N, np0);
    mpn_add(L(product1), L(product1), N, L(product0+1), N-1);

    np0 = pSecp256r1_np * product1[0];
    product2[N-1] = mpn_addmul_1(L(product1), L(mq), N, np0);
    mpn_add(L(product2), L(product2), N, L(product1+1), N-1);

    np0 = pSecp256r1_np * product2[0];
    product3[N-1] = mpn_addmul_1(L(product2), L(mq), N, np0);
    mpn_add(L(product3), L(product3), N, L(product2+1), N-1);

    np0 = pSecp256r1_np * product3[0];
    c = mpn_addmul_1(L(product3), L(mq), N, np0);

    mpn_copyi(L(pRawResult), L(product3+1), pSecp256r1_N64);

    if (c || mpn_cmp(L(pRawResult), L(mq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(mq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawToMontgomery(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA)
{
    pSecp256r1_rawMMul(pRawResult, pRawA, pSecp256r1_rawR2);
}

void pSecp256r1_rawFromMontgomery(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA)
{
    const mp_size_t  N = pSecp256r1_N64+1;
    const uint64_t  *mq = pSecp256r1_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N];
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    mpn_copyi(L(product0), L(pRawA), pSecp256r1_N64); product0[N-1] = 0;

    np0 = pSecp256r1_np * product0[0];
    product1[N-1] = mpn_addmul_1(L(product0), L(mq), N, np0);
    mpn_add(L(product1), L(product1), N, L(product0+1), N-1);

    np0 = pSecp256r1_np * product1[0];
    product2[N-1] = mpn_addmul_1(L(product1), L(mq), N, np0);
    mpn_add(L(product2), L(product2), N, L(product1+1), N-1);

    np0 = pSecp256r1_np * product2[0];
    product3[N-1] = mpn_addmul_1(L(product2), L(mq), N, np0);
    mpn_add(L(product3), L(product3), N, L(product2+1), N-1);

    np0 = pSecp256r1_np * product3[0];
    c = mpn_addmul_1(L(product3), L(mq), N, np0);

    mpn_copyi(L(pRawResult), L(product3+1), pSecp256r1_N64);

    if (c || mpn_cmp(L(pRawResult), L(mq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(mq), pSecp256r1_N64);
    }
}

int pSecp256r1_rawIsZero(const pSecp256r1RawElement rawA)
{
    return mpn_zero_p(L(rawA), pSecp256r1_N64) ? 1 : 0;
}

int pSecp256r1_rawCmp(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    return mpn_cmp(L(pRawA), L(pRawB), pSecp256r1_N64);
}

void pSecp256r1_rawSwap(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA)
{
    pSecp256r1RawElement temp;

    pSecp256r1_rawCopy(temp, pRawResult);
    pSecp256r1_rawCopy(pRawResult, pRawA);
    pSecp256r1_rawCopy(pRawA, temp);
}

void pSecp256r1_rawCopyS2L(pSecp256r1RawElement pRawResult, int64_t val)
{
    pRawResult[0] = val;

    pRawResult[1] = 0;
    pRawResult[2] = 0;
    pRawResult[3] = 0;

    if (val < 0) {

        pRawResult[1] = -1;
        pRawResult[2] = -1;
        pRawResult[3] = -1;

        mpn_add_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawAnd(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    mpn_and_n(L(pRawResult), L(pRawA), L(pRawB), pSecp256r1_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawOr(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    mpn_ior_n(L(pRawResult), L(pRawA), L(pRawB), pSecp256r1_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawXor(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    mpn_xor_n(L(pRawResult), L(pRawA), L(pRawB), pSecp256r1_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawShl(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b)
{
    uint64_t bit_shift  = b % 64;
    uint64_t word_shift = b / 64;
    uint64_t word_count = pSecp256r1_N64 - word_shift;

    mpn_copyi(L(r + word_shift), L(a), word_count);
    std::memset(r, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_lshift(L(r), L(r), pSecp256r1_N64, bit_shift);
    }

    r[3] &= lboMask;

    if (mpn_cmp(L(r), L(pSecp256r1_rawq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(r), L(r), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}

void pSecp256r1_rawShr(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b)
{
    const uint64_t bit_shift  = b % 64;
    const uint64_t word_shift = b / 64;
    const uint64_t word_count = pSecp256r1_N64 - word_shift;

    mpn_copyi(L(r), L(a + word_shift), word_count);
    std::memset(r + word_count, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_rshift(L(r), L(r), pSecp256r1_N64, bit_shift);
    }
}

void pSecp256r1_rawNot(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA)
{
    mpn_com(L(pRawResult), L(pRawA), pSecp256r1_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(pSecp256r1_rawq), pSecp256r1_N64);
    }
}
