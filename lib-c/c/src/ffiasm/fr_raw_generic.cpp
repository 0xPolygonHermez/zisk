#include "fr_element.hpp"
#include <gmp.h>
#include <cstring>

// uint64_t and mp_limb_t are distinct types on some platforms (e.g. macOS)
static_assert(sizeof(mp_limb_t) == sizeof(uint64_t), "64-bit GMP limbs required");
static inline mp_ptr    L(uint64_t *p)       { return reinterpret_cast<mp_ptr>(p); }
static inline mp_srcptr L(const uint64_t *p) { return reinterpret_cast<mp_srcptr>(p); }

static uint64_t     Fr_rawq[] = {0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029, 0};
static FrRawElement Fr_rawR2  = {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x0216d0b17f4e44a5};
static uint64_t     Fr_np     = 0xc2e1f593efffffff;
static uint64_t     lboMask   = 0x3fffffffffffffff;
static FrRawElement zero      = {0};


void Fr_rawAdd(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB)
{
    uint64_t carry = mpn_add_n(L(pRawResult), L(pRawA), L(pRawB), Fr_N64);

    if(carry || mpn_cmp(L(pRawResult), L(Fr_rawq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawAddLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_add_1(L(pRawResult), L(pRawA), Fr_N64, rawB);

    if(carry || mpn_cmp(L(pRawResult), L(Fr_rawq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawSub(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB)
{
    uint64_t carry = mpn_sub_n(L(pRawResult), L(pRawA), L(pRawB), Fr_N64);

    if(carry)
    {
        mpn_add_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawSubRegular(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
{
    mpn_sub_n(L(pRawResult), L(pRawA), L(pRawB), Fr_N64);
}

void Fr_rawSubSL(FrRawElement pRawResult, uint64_t rawA, FrRawElement pRawB)
{
    FrRawElement pRawA = {rawA};

    uint64_t carry = mpn_sub_n(L(pRawResult), L(pRawA), L(pRawB), Fr_N64);

    if(carry)
    {
        mpn_add_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawSubLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_sub_1(L(pRawResult), L(pRawA), Fr_N64, rawB);

    if(carry)
    {
        mpn_add_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawNeg(FrRawElement pRawResult, const FrRawElement pRawA)
{
    if (mpn_cmp(L(pRawA), L(zero), Fr_N64) != 0)
    {
        mpn_sub_n(L(pRawResult), L(Fr_rawq), L(pRawA), Fr_N64);
    }
    else
    {
        mpn_copyi(L(pRawResult), L(zero), Fr_N64);
    }
}

//  Substracts a long element and a short element form 0
void Fr_rawNegLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB)
{
    uint64_t carry1 = mpn_sub_1(L(pRawResult), L(Fr_rawq), Fr_N64, rawB);
    uint64_t carry2 = mpn_sub_n(L(pRawResult), L(pRawResult), L(pRawA), Fr_N64);

    if (carry1 || carry2)
    {
        mpn_add_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawCopy(FrRawElement pRawResult, const FrRawElement pRawA)
{
    memcpy(pRawResult, pRawA, sizeof(FrRawElement));
}

int Fr_rawIsEq(const FrRawElement pRawA, const FrRawElement pRawB)
{
    return mpn_cmp(L(pRawA), L(pRawB), Fr_N64) == 0;
}

void Fr_rawMMul(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB)
{
    const mp_size_t  N = Fr_N64+1;
    const uint64_t  *mq = Fr_rawq;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(L(product0), L(pRawB), Fr_N64, pRawA[0]);

    np0 = Fr_np * product0[0];
    product1[1] = mpn_addmul_1(L(product0), L(mq), N, np0);

    product1[N-1] = mpn_addmul_1(L(product1), L(pRawB), Fr_N64, pRawA[1]);
    mpn_add(L(product1), L(product1), N, L(product0+1), N-1);

    np0 = Fr_np * product1[0];
    product2[1] = mpn_addmul_1(L(product1), L(mq), N, np0);

    product2[N-1] = mpn_addmul_1(L(product2), L(pRawB), Fr_N64, pRawA[2]);
    mpn_add(L(product2), L(product2), N, L(product1+1), N-1);

    np0 = Fr_np * product2[0];
    product3[1] = mpn_addmul_1(L(product2), L(mq), N, np0);

    product3[N-1] = mpn_addmul_1(L(product3), L(pRawB), Fr_N64, pRawA[3]);
    mpn_add(L(product3), L(product3), N, L(product2+1), N-1);

    np0 = Fr_np * product3[0];
    mpn_addmul_1(L(product3), L(mq), N, np0);

    mpn_copyi(L(pRawResult), L(product3+1), Fr_N64);

    if (mpn_cmp(L(pRawResult), L(mq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(mq), Fr_N64);
    }
}

void Fr_rawMSquare(FrRawElement pRawResult, const FrRawElement pRawA)
{
    Fr_rawMMul(pRawResult, pRawA, pRawA);
}

void Fr_rawMMul1(FrRawElement pRawResult, const FrRawElement pRawA, uint64_t pRawB)
{
    const mp_size_t  N = Fr_N64+1;
    const uint64_t  *mq = Fr_rawq;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(L(product0), L(pRawA), Fr_N64, pRawB);

    np0 = Fr_np * product0[0];
    product1[1] = mpn_addmul_1(L(product0), L(mq), N, np0);
    mpn_add(L(product1), L(product1), N, L(product0+1), N-1);

    np0 = Fr_np * product1[0];
    product2[1] = mpn_addmul_1(L(product1), L(mq), N, np0);
    mpn_add(L(product2), L(product2), N, L(product1+1), N-1);

    np0 = Fr_np * product2[0];
    product3[1] = mpn_addmul_1(L(product2), L(mq), N, np0);
    mpn_add(L(product3), L(product3), N, L(product2+1), N-1);

    np0 = Fr_np * product3[0];
    mpn_addmul_1(L(product3), L(mq), N, np0);

    mpn_copyi(L(pRawResult), L(product3+1), Fr_N64);

    if (mpn_cmp(L(pRawResult), L(mq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(mq), Fr_N64);
    }
}

void Fr_rawToMontgomery(FrRawElement pRawResult, const FrRawElement pRawA)
{
    Fr_rawMMul(pRawResult, pRawA, Fr_rawR2);
}

void Fr_rawFromMontgomery(FrRawElement pRawResult, const FrRawElement pRawA)
{
    const mp_size_t  N = Fr_N64+1;
    const uint64_t  *mq = Fr_rawq;
    uint64_t  np0;
    uint64_t  product0[N];
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    mpn_copyi(L(product0), L(pRawA), Fr_N64); product0[N-1] = 0;

    np0 = Fr_np * product0[0];
    product1[1] = mpn_addmul_1(L(product0), L(mq), N, np0);
    mpn_add(L(product1), L(product1), N, L(product0+1), N-1);

    np0 = Fr_np * product1[0];
    product2[1] = mpn_addmul_1(L(product1), L(mq), N, np0);
    mpn_add(L(product2), L(product2), N, L(product1+1), N-1);

    np0 = Fr_np * product2[0];
    product3[1] = mpn_addmul_1(L(product2), L(mq), N, np0);
    mpn_add(L(product3), L(product3), N, L(product2+1), N-1);

    np0 = Fr_np * product3[0];
    mpn_addmul_1(L(product3), L(mq), N, np0);

    mpn_copyi(L(pRawResult), L(product3+1), Fr_N64);

    if (mpn_cmp(L(pRawResult), L(mq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(mq), Fr_N64);
    }
}

int Fr_rawIsZero(const FrRawElement rawA)
{
    return mpn_zero_p(L(rawA), Fr_N64) ? 1 : 0;
}

int Fr_rawCmp(FrRawElement pRawA, FrRawElement pRawB)
{
    return mpn_cmp(L(pRawA), L(pRawB), Fr_N64);
}

void Fr_rawSwap(FrRawElement pRawResult, FrRawElement pRawA)
{
    FrRawElement temp;

    Fr_rawCopy(temp, pRawResult);
    Fr_rawCopy(pRawResult, pRawA);
    Fr_rawCopy(pRawA, temp);
}

void Fr_rawCopyS2L(FrRawElement pRawResult, int64_t val)
{
    pRawResult[0] = val;

    pRawResult[1] = 0;
    pRawResult[2] = 0;
    pRawResult[3] = 0;

    if (val < 0) {

        pRawResult[1] = -1;
        pRawResult[2] = -1;
        pRawResult[3] = -1;

        mpn_add_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawAnd(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
{
    mpn_and_n(L(pRawResult), L(pRawA), L(pRawB), Fr_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(L(pRawResult), L(Fr_rawq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawOr(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
{
    mpn_ior_n(L(pRawResult), L(pRawA), L(pRawB), Fr_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(L(pRawResult), L(Fr_rawq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawXor(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
{
    mpn_xor_n(L(pRawResult), L(pRawA), L(pRawB), Fr_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(L(pRawResult), L(Fr_rawq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawShl(FrRawElement r, FrRawElement a, uint64_t b)
{
    uint64_t bit_shift  = b % 64;
    uint64_t word_shift = b / 64;
    uint64_t word_count = Fr_N64 - word_shift;

    mpn_copyi(L(r + word_shift), L(a), word_count);
    std::memset(r, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_lshift(L(r), L(r), Fr_N64, bit_shift);
    }

    r[3] &= lboMask;

    if (mpn_cmp(L(r), L(Fr_rawq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(r), L(r), L(Fr_rawq), Fr_N64);
    }
}

void Fr_rawShr(FrRawElement r, FrRawElement a, uint64_t b)
{
    const uint64_t bit_shift  = b % 64;
    const uint64_t word_shift = b / 64;
    const uint64_t word_count = Fr_N64 - word_shift;

    mpn_copyi(L(r), L(a + word_shift), word_count);
    std::memset(r + word_count, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_rshift(L(r), L(r), Fr_N64, bit_shift);
    }
}

void Fr_rawNot(FrRawElement pRawResult, FrRawElement pRawA)
{
    mpn_com(L(pRawResult), L(pRawA), Fr_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(L(pRawResult), L(Fr_rawq), Fr_N64) >= 0)
    {
        mpn_sub_n(L(pRawResult), L(pRawResult), L(Fr_rawq), Fr_N64);
    }
}
