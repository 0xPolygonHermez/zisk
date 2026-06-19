#include "fnec_element.hpp"
#include <gmp.h>
#include <cstring>

static uint64_t     Fnec_rawq[] = {0xbfd25e8cd0364141,0xbaaedce6af48a03b,0xfffffffffffffffe,0xffffffffffffffff, 0};
static FnecRawElement Fnec_rawR2  = {0x896cf21467d7d140,0x741496c20e7cf878,0xe697f5e45bcd07c6,0x9d671cd581c69bc5};
static uint64_t     Fnec_np     = 0x4b0dff665588b13f;
static uint64_t     lboMask   = 0xffffffffffffffff;
static FnecRawElement zero      = {0};


void Fnec_rawAdd(FnecRawElement pRawResult, const FnecRawElement pRawA, const FnecRawElement pRawB)
{
    uint64_t carry = mpn_add_n(pRawResult, pRawA, pRawB, Fnec_N64);

    if(carry || mpn_cmp(pRawResult, Fnec_rawq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawAddLS(FnecRawElement pRawResult, FnecRawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_add_1(pRawResult, pRawA, Fnec_N64, rawB);

    if(carry || mpn_cmp(pRawResult, Fnec_rawq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawSub(FnecRawElement pRawResult, const FnecRawElement pRawA, const FnecRawElement pRawB)
{
    uint64_t carry = mpn_sub_n(pRawResult, pRawA, pRawB, Fnec_N64);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawSubRegular(FnecRawElement pRawResult, FnecRawElement pRawA, FnecRawElement pRawB)
{
    mpn_sub_n(pRawResult, pRawA, pRawB, Fnec_N64);
}

void Fnec_rawSubSL(FnecRawElement pRawResult, uint64_t rawA, FnecRawElement pRawB)
{
    FnecRawElement pRawA = {rawA};

    uint64_t carry = mpn_sub_n(pRawResult, pRawA, pRawB, Fnec_N64);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawSubLS(FnecRawElement pRawResult, FnecRawElement pRawA, uint64_t rawB)
{
    uint64_t carry = mpn_sub_1(pRawResult, pRawA, Fnec_N64, rawB);

    if(carry)
    {
        mpn_add_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawNeg(FnecRawElement pRawResult, const FnecRawElement pRawA)
{
    if (mpn_cmp(pRawA, zero, Fnec_N64) != 0)
    {
        mpn_sub_n(pRawResult, Fnec_rawq, pRawA, Fnec_N64);
    }
    else
    {
        mpn_copyi(pRawResult, zero, Fnec_N64);
    }
}

//  Substracts a long element and a short element form 0
void Fnec_rawNegLS(FnecRawElement pRawResult, FnecRawElement pRawA, uint64_t rawB)
{
    uint64_t carry1 = mpn_sub_1(pRawResult, Fnec_rawq, Fnec_N64, rawB);
    uint64_t carry2 = mpn_sub_n(pRawResult, pRawResult, pRawA, Fnec_N64);

    if (carry1 || carry2)
    {
        mpn_add_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawCopy(FnecRawElement pRawResult, const FnecRawElement pRawA)
{
    memcpy(pRawResult, pRawA, sizeof(FnecRawElement));
}

int Fnec_rawIsEq(const FnecRawElement pRawA, const FnecRawElement pRawB)
{
    return mpn_cmp(pRawA, pRawB, Fnec_N64) == 0;
}

void Fnec_rawMMul(FnecRawElement pRawResult, const FnecRawElement pRawA, const FnecRawElement pRawB)
{
    const mp_size_t  N = Fnec_N64+1;
    const uint64_t  *mq = Fnec_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(product0, pRawB, Fnec_N64, pRawA[0]);

    np0 = Fnec_np * product0[0];
    product1[N-1] += mpn_addmul_1(product0, mq, N, np0);

    product1[N-1] += mpn_addmul_1(product1, pRawB, Fnec_N64, pRawA[1]);
    product2[N-1] = mpn_add(product1, product1, N, product0+1, N-1);

    np0 = Fnec_np * product1[0];
    product2[N-1] += mpn_addmul_1(product1, mq, N, np0);

    product2[N-1] += mpn_addmul_1(product2, pRawB, Fnec_N64, pRawA[2]);
    product3[N-1] = mpn_add(product2, product2, N, product1+1, N-1);

    np0 = Fnec_np * product2[0];
    product3[N-1] += mpn_addmul_1(product2, mq, N, np0);

    product3[N-1] += mpn_addmul_1(product3, pRawB, Fnec_N64, pRawA[3]);
    c = mpn_add(product3, product3, N, product2+1, N-1);

    np0 = Fnec_np * product3[0];
    c += mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, Fnec_N64);

    if (c || mpn_cmp(pRawResult, mq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, Fnec_N64);
    }
}

void Fnec_rawMSquare(FnecRawElement pRawResult, const FnecRawElement pRawA)
{
    Fnec_rawMMul(pRawResult, pRawA, pRawA);
}

void Fnec_rawMMul1(FnecRawElement pRawResult, const FnecRawElement pRawA, uint64_t pRawB)
{
    const mp_size_t  N = Fnec_N64+1;
    const uint64_t  *mq = Fnec_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N] = {0};
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    product0[N-1] = mpn_mul_1(product0, pRawA, Fnec_N64, pRawB);

    np0 = Fnec_np * product0[0];
    product1[N-1] = mpn_addmul_1(product0, mq, N, np0);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = Fnec_np * product1[0];
    product2[N-1] = mpn_addmul_1(product1, mq, N, np0);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = Fnec_np * product2[0];
    product3[N-1] = mpn_addmul_1(product2, mq, N, np0);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = Fnec_np * product3[0];
    c = mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, Fnec_N64);

    if (c || mpn_cmp(pRawResult, mq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, Fnec_N64);
    }
}

void Fnec_rawToMontgomery(FnecRawElement pRawResult, const FnecRawElement pRawA)
{
    Fnec_rawMMul(pRawResult, pRawA, Fnec_rawR2);
}

void Fnec_rawFromMontgomery(FnecRawElement pRawResult, const FnecRawElement pRawA)
{
    const mp_size_t  N = Fnec_N64+1;
    const uint64_t  *mq = Fnec_rawq;

    uint64_t  c = 0;
    uint64_t  np0;
    uint64_t  product0[N];
    uint64_t  product1[N] = {0};
    uint64_t  product2[N] = {0};
    uint64_t  product3[N] = {0};

    mpn_copyi(product0, pRawA, Fnec_N64); product0[N-1] = 0;

    np0 = Fnec_np * product0[0];
    product1[N-1] = mpn_addmul_1(product0, mq, N, np0);
    mpn_add(product1, product1, N, product0+1, N-1);

    np0 = Fnec_np * product1[0];
    product2[N-1] = mpn_addmul_1(product1, mq, N, np0);
    mpn_add(product2, product2, N, product1+1, N-1);

    np0 = Fnec_np * product2[0];
    product3[N-1] = mpn_addmul_1(product2, mq, N, np0);
    mpn_add(product3, product3, N, product2+1, N-1);

    np0 = Fnec_np * product3[0];
    c = mpn_addmul_1(product3, mq, N, np0);

    mpn_copyi(pRawResult,  product3+1, Fnec_N64);

    if (c || mpn_cmp(pRawResult, mq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, mq, Fnec_N64);
    }
}

int Fnec_rawIsZero(const FnecRawElement rawA)
{
    return mpn_zero_p(rawA, Fnec_N64) ? 1 : 0;
}

int Fnec_rawCmp(FnecRawElement pRawA, FnecRawElement pRawB)
{
    return mpn_cmp(pRawA, pRawB, Fnec_N64);
}

void Fnec_rawSwap(FnecRawElement pRawResult, FnecRawElement pRawA)
{
    FnecRawElement temp;

    Fnec_rawCopy(temp, pRawResult);
    Fnec_rawCopy(pRawResult, pRawA);
    Fnec_rawCopy(pRawA, temp);
}

void Fnec_rawCopyS2L(FnecRawElement pRawResult, int64_t val)
{
    pRawResult[0] = val;

    pRawResult[1] = 0;
    pRawResult[2] = 0;
    pRawResult[3] = 0;

    if (val < 0) {

        pRawResult[1] = -1;
        pRawResult[2] = -1;
        pRawResult[3] = -1;

        mpn_add_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawAnd(FnecRawElement pRawResult, FnecRawElement pRawA, FnecRawElement pRawB)
{
    mpn_and_n(pRawResult, pRawA, pRawB, Fnec_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, Fnec_rawq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawOr(FnecRawElement pRawResult, FnecRawElement pRawA, FnecRawElement pRawB)
{
    mpn_ior_n(pRawResult, pRawA, pRawB, Fnec_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, Fnec_rawq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawXor(FnecRawElement pRawResult, FnecRawElement pRawA, FnecRawElement pRawB)
{
    mpn_xor_n(pRawResult, pRawA, pRawB, Fnec_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, Fnec_rawq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawShl(FnecRawElement r, FnecRawElement a, uint64_t b)
{
    uint64_t bit_shift  = b % 64;
    uint64_t word_shift = b / 64;
    uint64_t word_count = Fnec_N64 - word_shift;

    mpn_copyi(r + word_shift, a, word_count);
    std::memset(r, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_lshift(r, r, Fnec_N64, bit_shift);
    }

    r[3] &= lboMask;

    if (mpn_cmp(r, Fnec_rawq, Fnec_N64) >= 0)
    {
        mpn_sub_n(r, r, Fnec_rawq, Fnec_N64);
    }
}

void Fnec_rawShr(FnecRawElement r, FnecRawElement a, uint64_t b)
{
    const uint64_t bit_shift  = b % 64;
    const uint64_t word_shift = b / 64;
    const uint64_t word_count = Fnec_N64 - word_shift;

    mpn_copyi(r, a + word_shift, word_count);
    std::memset(r + word_count, 0, word_shift * sizeof(uint64_t));

    if (bit_shift)
    {
        mpn_rshift(r, r, Fnec_N64, bit_shift);
    }
}

void Fnec_rawNot(FnecRawElement pRawResult, FnecRawElement pRawA)
{
    mpn_com(pRawResult, pRawA, Fnec_N64);

    pRawResult[3] &= lboMask;

    if (mpn_cmp(pRawResult, Fnec_rawq, Fnec_N64) >= 0)
    {
        mpn_sub_n(pRawResult, pRawResult, Fnec_rawq, Fnec_N64);
    }
}
