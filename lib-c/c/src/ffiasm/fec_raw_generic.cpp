#include "fec_element.hpp"
#include <gmp.h>
#include <cstring>

static uint64_t     Fec_rawq[] = {0xfffffffefffffc2f,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff, 0};
static FecRawElement Fec_rawR2  = {0x000007a2000e90a1,0x0000000000000001,0x0000000000000000,0x0000000000000000};
static uint64_t     Fec_np     = 0xd838091dd2253531;
static uint64_t     lboMask   = 0xffffffffffffffff;
static FecRawElement zero      = {0};


void Fec_rawAdd(FecRawElement pRawResult, const FecRawElement pRawA, const FecRawElement pRawB)
{
    uint64_t carry = mpn_add_n(pRawResult, pRawA, pRawB, Fec_N64);

    if(carry || mpn_cmp(pRawResult, Fec_rawq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawAddLS(FecRawElement pRawResult, FecRawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_add_1(pRawResult, pRawA, Fec_N64, rawB);

    if(carry || mpn_cmp(pRawResult, Fec_rawq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawSub(FecRawElement pRawResult, const FecRawElement pRawA, const FecRawElement pRawB)
{
    uint64_t carry = mpn_sub_n(pRawResult, pRawA, pRawB, Fec_N64);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawSubRegular(FecRawElement pRawResult, FecRawElement pRawA, FecRawElement pRawB)
{
    mpn_sub_n(pRawResult, pRawA, pRawB, Fec_N64);
}

void Fec_rawSubSL(FecRawElement pRawResult, uint64_t rawA, FecRawElement pRawB)
{
    FecRawElement pRawA = {rawA};

    uint64_t carry = mpn_sub_n(pRawResult, pRawA, pRawB, Fec_N64);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawSubLS(FecRawElement pRawResult, FecRawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_sub_1(pRawResult, pRawA, Fec_N64, rawB);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawNeg(FecRawElement pRawResult, const FecRawElement pRawA)
{
    if (mpn_cmp(pRawA, zero, Fec_N64) != 0)
    {
        mpn_sub_n(pRawResult, Fec_rawq, pRawA, Fec_N64);
    }
    else
    {
        mpn_copyi(pRawResult, zero, Fec_N64);
    }
}

//  Substracts a long element and a short element form 0
void Fec_rawNegLS(FecRawElement pRawResult, FecRawElement pRawA, uint64_t rawB)
{
    uint64_t carry1 = mpn_sub_1(pRawResult, Fec_rawq, Fec_N64, rawB);
    uint64_t carry2 = mpn_sub_n(pRawResult, pRawResult, pRawA, Fec_N64);

    if (carry1 || carry2)
    {
        mpn_add_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawCopy(FecRawElement pRawResult, const FecRawElement pRawA)
{
    memcpy(pRawResult, pRawA, sizeof(FecRawElement));
}

int Fec_rawIsEq(const FecRawElement pRawA, const FecRawElement pRawB)
{
    return mpn_cmp(pRawA, pRawB, Fec_N64) == 0;
}

void Fec_rawMMul(FecRawElement pRawResult, const FecRawElement pRawA, const FecRawElement pRawB)
{
    const mp_size_t  N = Fec_N64+1;
    const uint64_t  *mq = Fec_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(product0, pRawB, Fec_N64, pRawA[0]);

    np0 = Fec_np * product0[0];
    product1[N-1] += mpn_addmul_1(product0, mq, N, np0);

    product1[N-1] += mpn_addmul_1(product1, pRawB, Fec_N64, pRawA[1]);
    product2[N-1] = mpn_add(product1, product1, N, product0+1, N-1);

    np0 = Fec_np * product1[0];
    product2[N-1] += mpn_addmul_1(product1, mq, N, np0);

    product2[N-1] += mpn_addmul_1(product2, pRawB, Fec_N64, pRawA[2]);
    product3[N-1] = mpn_add(product2, product2, N, product1+1, N-1);

    np0 = Fec_np * product2[0];
    product3[N-1] += mpn_addmul_1(product2, mq, N, np0);

    product3[N-1] += mpn_addmul_1(product3, pRawB, Fec_N64, pRawA[3]);
    c = mpn_add(product3, product3, N, product2+1, N-1);

    np0 = Fec_np * product3[0];
    c += mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, Fec_N64);

    if (c || mpn_cmp(pRawResult, mq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, Fec_N64);
    }
}

void Fec_rawMSquare(FecRawElement pRawResult, const FecRawElement pRawA)
{
    Fec_rawMMul(pRawResult, pRawA, pRawA);
}

void Fec_rawMMul1(FecRawElement pRawResult, const FecRawElement pRawA, uint64_t pRawB)
{
    const mp_size_t  N = Fec_N64+1;
    const uint64_t  *mq = Fec_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(product0, pRawA, Fec_N64, pRawB);

    np0 = Fec_np * product0[0];
    product1[N-1] = mpn_addmul_1(product0, mq, N, np0);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = Fec_np * product1[0];
    product2[N-1] = mpn_addmul_1(product1, mq, N, np0);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = Fec_np * product2[0];
    product3[N-1] = mpn_addmul_1(product2, mq, N, np0);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = Fec_np * product3[0];
    c = mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, Fec_N64);

    if (c || mpn_cmp(pRawResult, mq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, Fec_N64);
    }
}

void Fec_rawToMontgomery(FecRawElement pRawResult, const FecRawElement pRawA)
{
    Fec_rawMMul(pRawResult, pRawA, Fec_rawR2);
}

void Fec_rawFromMontgomery(FecRawElement pRawResult, const FecRawElement pRawA)
{
    const mp_size_t  N = Fec_N64+1;
    const uint64_t  *mq = Fec_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N];
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    mpn_copyi(product0, pRawA, Fec_N64); product0[N-1] = 0;

    np0 = Fec_np * product0[0];
    product1[N-1] = mpn_addmul_1(product0, mq, N, np0);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = Fec_np * product1[0];
    product2[N-1] = mpn_addmul_1(product1, mq, N, np0);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = Fec_np * product2[0];
    product3[N-1] = mpn_addmul_1(product2, mq, N, np0);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = Fec_np * product3[0];
    c = mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, Fec_N64);

    if (c || mpn_cmp(pRawResult, mq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, Fec_N64);
    }
}

int Fec_rawIsZero(const FecRawElement rawA)
{
    return mpn_zero_p(rawA, Fec_N64) ? 1 : 0;
}

int Fec_rawCmp(FecRawElement pRawA, FecRawElement pRawB)
{
    return mpn_cmp(pRawA, pRawB, Fec_N64);
}

void Fec_rawSwap(FecRawElement pRawResult, FecRawElement pRawA)
{
    FecRawElement temp;

    Fec_rawCopy(temp, pRawResult);
    Fec_rawCopy(pRawResult, pRawA);
    Fec_rawCopy(pRawA, temp);
}

void Fec_rawCopyS2L(FecRawElement pRawResult, int64_t val)
{
    pRawResult[0] = val;

    pRawResult[1] = 0;
    pRawResult[2] = 0;
    pRawResult[3] = 0;

    if (val < 0) {

        pRawResult[1] = -1;
        pRawResult[2] = -1;
        pRawResult[3] = -1;

        mpn_add_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawAnd(FecRawElement pRawResult, FecRawElement pRawA, FecRawElement pRawB)
{
    mpn_and_n(pRawResult, pRawA, pRawB, Fec_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, Fec_rawq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawOr(FecRawElement pRawResult, FecRawElement pRawA, FecRawElement pRawB)
{
    mpn_ior_n(pRawResult, pRawA, pRawB, Fec_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, Fec_rawq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawXor(FecRawElement pRawResult, FecRawElement pRawA, FecRawElement pRawB)
{
    mpn_xor_n(pRawResult, pRawA, pRawB, Fec_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, Fec_rawq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}

void Fec_rawShl(FecRawElement r, FecRawElement a, uint64_t b)
{
    uint64_t bit_shift  = b % 64;
    uint64_t word_shift = b / 64;
    uint64_t word_count = Fec_N64 - word_shift;

    mpn_copyi(r + word_shift, a, word_count);
    std::memset(r, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_lshift(r, r, Fec_N64, bit_shift);
    }

    r[3] &= lboMask;

    if (mpn_cmp(r, Fec_rawq, Fec_N64) >= 0)
    {
        mpn_sub_n(r, r, Fec_rawq, Fec_N64);
    }
}

void Fec_rawShr(FecRawElement r, FecRawElement a, uint64_t b)
{
    const uint64_t bit_shift  = b % 64;
    const uint64_t word_shift = b / 64;
    const uint64_t word_count = Fec_N64 - word_shift;

    mpn_copyi(r, a + word_shift, word_count);
    std::memset(r + word_count, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_rshift(r, r, Fec_N64, bit_shift);
    }
}

void Fec_rawNot(FecRawElement pRawResult, FecRawElement pRawA)
{
    mpn_com(pRawResult, pRawA, Fec_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, Fec_rawq, Fec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fec_rawq, Fec_N64);
    }
}
