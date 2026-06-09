#include "nsecp256r1_element.hpp"
#include <gmp.h>
#include <cstring>

static uint64_t     nSecp256r1_rawq[] = {0xf3b9cac2fc632551,0xbce6faada7179e84,0xffffffffffffffff,0xffffffff00000000, 0};
static nSecp256r1RawElement nSecp256r1_rawR2  = {0x83244c95be79eea2,0x4699799c49bd6fa6,0x2845b2392b6bec59,0x66e12d94f3d95620};
static uint64_t     nSecp256r1_np     = 0xccd1c8aaee00bc4f;
static uint64_t     lboMask   = 0xffffffffffffffff;
static nSecp256r1RawElement zero      = {0};


void nSecp256r1_rawAdd(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA, const nSecp256r1RawElement pRawB)
{
    uint64_t carry = mpn_add_n(pRawResult, pRawA, pRawB, nSecp256r1_N64);

    if(carry || mpn_cmp(pRawResult, nSecp256r1_rawq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawAddLS(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_add_1(pRawResult, pRawA, nSecp256r1_N64, rawB);

    if(carry || mpn_cmp(pRawResult, nSecp256r1_rawq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawSub(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA, const nSecp256r1RawElement pRawB)
{
    uint64_t carry = mpn_sub_n(pRawResult, pRawA, pRawB, nSecp256r1_N64);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawSubRegular(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    mpn_sub_n(pRawResult, pRawA, pRawB, nSecp256r1_N64);
}

void nSecp256r1_rawSubSL(nSecp256r1RawElement pRawResult, uint64_t rawA, nSecp256r1RawElement pRawB)
{
    nSecp256r1RawElement pRawA = {rawA};

    uint64_t carry = mpn_sub_n(pRawResult, pRawA, pRawB, nSecp256r1_N64);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawSubLS(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_sub_1(pRawResult, pRawA, nSecp256r1_N64, rawB);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawNeg(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA)
{
    if (mpn_cmp(pRawA, zero, nSecp256r1_N64) != 0)
    {
        mpn_sub_n(pRawResult, nSecp256r1_rawq, pRawA, nSecp256r1_N64);
    }
    else
    {
        mpn_copyi(pRawResult, zero, nSecp256r1_N64);
    }
}

//  Substracts a long element and a short element form 0
void nSecp256r1_rawNegLS(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA, uint64_t rawB)
{
    uint64_t carry1 = mpn_sub_1(pRawResult, nSecp256r1_rawq, nSecp256r1_N64, rawB);
    uint64_t carry2 = mpn_sub_n(pRawResult, pRawResult, pRawA, nSecp256r1_N64);

    if (carry1 || carry2)
    {
        mpn_add_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawCopy(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA)
{
    memcpy(pRawResult, pRawA, sizeof(nSecp256r1RawElement));
}

int nSecp256r1_rawIsEq(const nSecp256r1RawElement pRawA, const nSecp256r1RawElement pRawB)
{
    return mpn_cmp(pRawA, pRawB, nSecp256r1_N64) == 0;
}

void nSecp256r1_rawMMul(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA, const nSecp256r1RawElement pRawB)
{
    const mp_size_t  N = nSecp256r1_N64+1;
    const uint64_t  *mq = nSecp256r1_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(product0, pRawB, nSecp256r1_N64, pRawA[0]);

    np0 = nSecp256r1_np * product0[0];
    product1[N-1] += mpn_addmul_1(product0, mq, N, np0);

    product1[N-1] += mpn_addmul_1(product1, pRawB, nSecp256r1_N64, pRawA[1]);
    product2[N-1] = mpn_add(product1, product1, N, product0+1, N-1);

    np0 = nSecp256r1_np * product1[0];
    product2[N-1] += mpn_addmul_1(product1, mq, N, np0);

    product2[N-1] += mpn_addmul_1(product2, pRawB, nSecp256r1_N64, pRawA[2]);
    product3[N-1] = mpn_add(product2, product2, N, product1+1, N-1);

    np0 = nSecp256r1_np * product2[0];
    product3[N-1] += mpn_addmul_1(product2, mq, N, np0);

    product3[N-1] += mpn_addmul_1(product3, pRawB, nSecp256r1_N64, pRawA[3]);
    c = mpn_add(product3, product3, N, product2+1, N-1);

    np0 = nSecp256r1_np * product3[0];
    c += mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, nSecp256r1_N64);

    if (c || mpn_cmp(pRawResult, mq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawMSquare(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA)
{
    nSecp256r1_rawMMul(pRawResult, pRawA, pRawA);
}

void nSecp256r1_rawMMul1(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA, uint64_t pRawB)
{
    const mp_size_t  N = nSecp256r1_N64+1;
    const uint64_t  *mq = nSecp256r1_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(product0, pRawA, nSecp256r1_N64, pRawB);

    np0 = nSecp256r1_np * product0[0];
    product1[N-1] = mpn_addmul_1(product0, mq, N, np0);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = nSecp256r1_np * product1[0];
    product2[N-1] = mpn_addmul_1(product1, mq, N, np0);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = nSecp256r1_np * product2[0];
    product3[N-1] = mpn_addmul_1(product2, mq, N, np0);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = nSecp256r1_np * product3[0];
    c = mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, nSecp256r1_N64);

    if (c || mpn_cmp(pRawResult, mq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawToMontgomery(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA)
{
    nSecp256r1_rawMMul(pRawResult, pRawA, nSecp256r1_rawR2);
}

void nSecp256r1_rawFromMontgomery(nSecp256r1RawElement pRawResult, const nSecp256r1RawElement pRawA)
{
    const mp_size_t  N = nSecp256r1_N64+1;
    const uint64_t  *mq = nSecp256r1_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N];
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    mpn_copyi(product0, pRawA, nSecp256r1_N64); product0[N-1] = 0;

    np0 = nSecp256r1_np * product0[0];
    product1[N-1] = mpn_addmul_1(product0, mq, N, np0);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = nSecp256r1_np * product1[0];
    product2[N-1] = mpn_addmul_1(product1, mq, N, np0);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = nSecp256r1_np * product2[0];
    product3[N-1] = mpn_addmul_1(product2, mq, N, np0);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = nSecp256r1_np * product3[0];
    c = mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, nSecp256r1_N64);

    if (c || mpn_cmp(pRawResult, mq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, nSecp256r1_N64);
    }
}

int nSecp256r1_rawIsZero(const nSecp256r1RawElement rawA)
{
    return mpn_zero_p(rawA, nSecp256r1_N64) ? 1 : 0;
}

int nSecp256r1_rawCmp(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    return mpn_cmp(pRawA, pRawB, nSecp256r1_N64);
}

void nSecp256r1_rawSwap(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA)
{
    nSecp256r1RawElement temp;

    nSecp256r1_rawCopy(temp, pRawResult);
    nSecp256r1_rawCopy(pRawResult, pRawA);
    nSecp256r1_rawCopy(pRawA, temp);
}

void nSecp256r1_rawCopyS2L(nSecp256r1RawElement pRawResult, int64_t val)
{
    pRawResult[0] = val;

    pRawResult[1] = 0;
    pRawResult[2] = 0;
    pRawResult[3] = 0;

    if (val < 0) {

        pRawResult[1] = -1;
        pRawResult[2] = -1;
        pRawResult[3] = -1;

        mpn_add_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawAnd(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    mpn_and_n(pRawResult, pRawA, pRawB, nSecp256r1_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, nSecp256r1_rawq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawOr(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    mpn_ior_n(pRawResult, pRawA, pRawB, nSecp256r1_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, nSecp256r1_rawq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawXor(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    mpn_xor_n(pRawResult, pRawA, pRawB, nSecp256r1_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, nSecp256r1_rawq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawShl(nSecp256r1RawElement r, nSecp256r1RawElement a, uint64_t b)
{
    uint64_t bit_shift  = b % 64;
    uint64_t word_shift = b / 64;
    uint64_t word_count = nSecp256r1_N64 - word_shift;

    mpn_copyi(r + word_shift, a, word_count);
    std::memset(r, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_lshift(r, r, nSecp256r1_N64, bit_shift);
    }

    r[3] &= lboMask;

    if (mpn_cmp(r, nSecp256r1_rawq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(r, r, nSecp256r1_rawq, nSecp256r1_N64);
    }
}

void nSecp256r1_rawShr(nSecp256r1RawElement r, nSecp256r1RawElement a, uint64_t b)
{
    const uint64_t bit_shift  = b % 64;
    const uint64_t word_shift = b / 64;
    const uint64_t word_count = nSecp256r1_N64 - word_shift;

    mpn_copyi(r, a + word_shift, word_count);
    std::memset(r + word_count, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_rshift(r, r, nSecp256r1_N64, bit_shift);
    }
}

void nSecp256r1_rawNot(nSecp256r1RawElement pRawResult, nSecp256r1RawElement pRawA)
{
    mpn_com(pRawResult, pRawA, nSecp256r1_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, nSecp256r1_rawq, nSecp256r1_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, nSecp256r1_rawq, nSecp256r1_N64);
    }
}
