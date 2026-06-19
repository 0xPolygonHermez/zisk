#include "bls12_381_384_element.hpp"
#include <gmp.h>
#include <cstring>

static uint64_t     BLS12_381_384_rawq[] = {0xb9feffffffffaaab,0x1eabfffeb153ffff,0x6730d2a0f6b0f624,0x64774b84f38512bf,0x4b1ba7b6434bacd7,0x1a0111ea397fe69a, 0};
static BLS12_381_384RawElement BLS12_381_384_rawR2  = {0xf4df1f341c341746,0x0a76e6a609d104f1,0x8de5476c4c95b6d5,0x67eb88a9939d83c0,0x9a793e85b519952d,0x11988fe592cae3aa};
static uint64_t     BLS12_381_384_np     = 0x89f3fffcfffcfffd;
static uint64_t     lboMask   = 0x1fffffffffffffff;
static BLS12_381_384RawElement zero      = {0};


void BLS12_381_384_rawAdd(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA, const BLS12_381_384RawElement pRawB)
{
    uint64_t carry = mpn_add_n(pRawResult, pRawA, pRawB, BLS12_381_384_N64);

    if(carry || mpn_cmp(pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawAddLS(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_add_1(pRawResult, pRawA, BLS12_381_384_N64, rawB);

    if(carry || mpn_cmp(pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawSub(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA, const BLS12_381_384RawElement pRawB)
{
    uint64_t carry = mpn_sub_n(pRawResult, pRawA, pRawB, BLS12_381_384_N64);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawSubRegular(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA, BLS12_381_384RawElement pRawB)
{
    mpn_sub_n(pRawResult, pRawA, pRawB, BLS12_381_384_N64);
}

void BLS12_381_384_rawSubSL(BLS12_381_384RawElement pRawResult, uint64_t rawA, BLS12_381_384RawElement pRawB)
{
    BLS12_381_384RawElement pRawA = {rawA};

    uint64_t carry = mpn_sub_n(pRawResult, pRawA, pRawB, BLS12_381_384_N64);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawSubLS(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_sub_1(pRawResult, pRawA, BLS12_381_384_N64, rawB);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawNeg(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA)
{
    if (mpn_cmp(pRawA, zero, BLS12_381_384_N64) != 0)
    {
        mpn_sub_n(pRawResult, BLS12_381_384_rawq, pRawA, BLS12_381_384_N64);
    }
    else
    {
        mpn_copyi(pRawResult, zero, BLS12_381_384_N64);
    }
}

//  Substracts a long element and a short element form 0
void BLS12_381_384_rawNegLS(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA, uint64_t rawB)
{
    uint64_t carry1 = mpn_sub_1(pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64, rawB);
    uint64_t carry2 = mpn_sub_n(pRawResult, pRawResult, pRawA, BLS12_381_384_N64);

    if (carry1 || carry2)
    {
        mpn_add_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawCopy(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA)
{
    memcpy(pRawResult, pRawA, sizeof(BLS12_381_384RawElement));
}

int BLS12_381_384_rawIsEq(const BLS12_381_384RawElement pRawA, const BLS12_381_384RawElement pRawB)
{
    return mpn_cmp(pRawA, pRawB, BLS12_381_384_N64) == 0;
}

void BLS12_381_384_rawMMul(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA, const BLS12_381_384RawElement pRawB)
{
    const mp_size_t  N = BLS12_381_384_N64+1;
    const uint64_t  *mq = BLS12_381_384_rawq;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};
    uint64_t  product4[N] = {0};
    uint64_t  product5[N] = {0};

    product0[N-1] = mpn_mul_1(product0, pRawB, BLS12_381_384_N64, pRawA[0]);

    np0 = BLS12_381_384_np * product0[0];
    product1[1] = mpn_addmul_1(product0, mq, N, np0);

    product1[N-1] = mpn_addmul_1(product1, pRawB, BLS12_381_384_N64, pRawA[1]);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = BLS12_381_384_np * product1[0];
    product2[1] = mpn_addmul_1(product1, mq, N, np0);

    product2[N-1] = mpn_addmul_1(product2, pRawB, BLS12_381_384_N64, pRawA[2]);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = BLS12_381_384_np * product2[0];
    product3[1] = mpn_addmul_1(product2, mq, N, np0);

    product3[N-1] = mpn_addmul_1(product3, pRawB, BLS12_381_384_N64, pRawA[3]);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = BLS12_381_384_np * product3[0];
    product4[1] = mpn_addmul_1(product3, mq, N, np0);

    product4[N-1] = mpn_addmul_1(product4, pRawB, BLS12_381_384_N64, pRawA[4]);
    mpn_add(product4, product4, N, product3+1, N-1);

    np0 = BLS12_381_384_np * product4[0];
    product5[1] = mpn_addmul_1(product4, mq, N, np0);

    product5[N-1] = mpn_addmul_1(product5, pRawB, BLS12_381_384_N64, pRawA[5]);
    mpn_add(product5, product5, N, product4+1, N-1);

    np0 = BLS12_381_384_np * product5[0];
    mpn_addmul_1(product5, mq, N, np0);

    mpn_copyi(pRawResult,  product5+1, BLS12_381_384_N64);

    if (mpn_cmp(pRawResult, mq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawMSquare(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA)
{
    BLS12_381_384_rawMMul(pRawResult, pRawA, pRawA);
}

void BLS12_381_384_rawMMul1(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA, uint64_t pRawB)
{
    const mp_size_t  N = BLS12_381_384_N64+1;
    const uint64_t  *mq = BLS12_381_384_rawq;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};
    uint64_t  product4[N] = {0};
    uint64_t  product5[N] = {0};

    product0[N-1] = mpn_mul_1(product0, pRawA, BLS12_381_384_N64, pRawB);

    np0 = BLS12_381_384_np * product0[0];
    product1[1] = mpn_addmul_1(product0, mq, N, np0);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = BLS12_381_384_np * product1[0];
    product2[1] = mpn_addmul_1(product1, mq, N, np0);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = BLS12_381_384_np * product2[0];
    product3[1] = mpn_addmul_1(product2, mq, N, np0);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = BLS12_381_384_np * product3[0];
    product4[1] = mpn_addmul_1(product3, mq, N, np0);
    mpn_add(product4, product4, N, product3+1, N-1);

    np0 = BLS12_381_384_np * product4[0];
    product5[1] = mpn_addmul_1(product4, mq, N, np0);
    mpn_add(product5, product5, N, product4+1, N-1);

    np0 = BLS12_381_384_np * product5[0];
    mpn_addmul_1(product5, mq, N, np0);

    mpn_copyi(pRawResult,  product5+1, BLS12_381_384_N64);

    if (mpn_cmp(pRawResult, mq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawToMontgomery(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA)
{
    BLS12_381_384_rawMMul(pRawResult, pRawA, BLS12_381_384_rawR2);
}

void BLS12_381_384_rawFromMontgomery(BLS12_381_384RawElement pRawResult, const BLS12_381_384RawElement pRawA)
{
    const mp_size_t  N = BLS12_381_384_N64+1;
    const uint64_t  *mq = BLS12_381_384_rawq;
    uint64_t  np0;
    uint64_t  product0[N];
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};
    uint64_t  product4[N] = {0};
    uint64_t  product5[N] = {0};

    mpn_copyi(product0, pRawA, BLS12_381_384_N64); product0[N-1] = 0;

    np0 = BLS12_381_384_np * product0[0];
    product1[1] = mpn_addmul_1(product0, mq, N, np0);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = BLS12_381_384_np * product1[0];
    product2[1] = mpn_addmul_1(product1, mq, N, np0);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = BLS12_381_384_np * product2[0];
    product3[1] = mpn_addmul_1(product2, mq, N, np0);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = BLS12_381_384_np * product3[0];
    product4[1] = mpn_addmul_1(product3, mq, N, np0);
    mpn_add(product4, product4, N, product3+1, N-1);

    np0 = BLS12_381_384_np * product4[0];
    product5[1] = mpn_addmul_1(product4, mq, N, np0);
    mpn_add(product5, product5, N, product4+1, N-1);

    np0 = BLS12_381_384_np * product5[0];
    mpn_addmul_1(product5, mq, N, np0);

    mpn_copyi(pRawResult,  product5+1, BLS12_381_384_N64);

    if (mpn_cmp(pRawResult, mq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, BLS12_381_384_N64);
    }
}

int BLS12_381_384_rawIsZero(const BLS12_381_384RawElement rawA)
{
    return mpn_zero_p(rawA, BLS12_381_384_N64) ? 1 : 0;
}

int BLS12_381_384_rawCmp(BLS12_381_384RawElement pRawA, BLS12_381_384RawElement pRawB)
{
    return mpn_cmp(pRawA, pRawB, BLS12_381_384_N64);
}

void BLS12_381_384_rawSwap(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA)
{
    BLS12_381_384RawElement temp;

    BLS12_381_384_rawCopy(temp, pRawResult);
    BLS12_381_384_rawCopy(pRawResult, pRawA);
    BLS12_381_384_rawCopy(pRawA, temp);
}

void BLS12_381_384_rawCopyS2L(BLS12_381_384RawElement pRawResult, int64_t val)
{
    pRawResult[0] = val;

    pRawResult[1] = 0;
    pRawResult[2] = 0;
    pRawResult[3] = 0;
    pRawResult[4] = 0;
    pRawResult[5] = 0;

    if (val < 0) {

        pRawResult[1] = -1;
        pRawResult[2] = -1;
        pRawResult[3] = -1;
        pRawResult[4] = -1;
        pRawResult[5] = -1;

        mpn_add_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawAnd(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA, BLS12_381_384RawElement pRawB)
{
    mpn_and_n(pRawResult, pRawA, pRawB, BLS12_381_384_N64);

    pRawResult[5] &= lboMask;

    if (mpn_cmp(pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawOr(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA, BLS12_381_384RawElement pRawB)
{
    mpn_ior_n(pRawResult, pRawA, pRawB, BLS12_381_384_N64);

    pRawResult[5] &= lboMask;

    if (mpn_cmp(pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawXor(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA, BLS12_381_384RawElement pRawB)
{
    mpn_xor_n(pRawResult, pRawA, pRawB, BLS12_381_384_N64);

    pRawResult[5] &= lboMask;

    if (mpn_cmp(pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawShl(BLS12_381_384RawElement r, BLS12_381_384RawElement a, uint64_t b)
{
    uint64_t bit_shift  = b % 64;
    uint64_t word_shift = b / 64;
    uint64_t word_count = BLS12_381_384_N64 - word_shift;

    mpn_copyi(r + word_shift, a, word_count);
    std::memset(r, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_lshift(r, r, BLS12_381_384_N64, bit_shift);
    }

    r[5] &= lboMask;

    if (mpn_cmp(r, BLS12_381_384_rawq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(r, r, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}

void BLS12_381_384_rawShr(BLS12_381_384RawElement r, BLS12_381_384RawElement a, uint64_t b)
{
    const uint64_t bit_shift  = b % 64;
    const uint64_t word_shift = b / 64;
    const uint64_t word_count = BLS12_381_384_N64 - word_shift;

    mpn_copyi(r, a + word_shift, word_count);
    std::memset(r + word_count, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_rshift(r, r, BLS12_381_384_N64, bit_shift);
    }
}

void BLS12_381_384_rawNot(BLS12_381_384RawElement pRawResult, BLS12_381_384RawElement pRawA)
{
    mpn_com(pRawResult, pRawA, BLS12_381_384_N64);

    pRawResult[5] &= lboMask;

    if (mpn_cmp(pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, BLS12_381_384_rawq, BLS12_381_384_N64);
    }
}
