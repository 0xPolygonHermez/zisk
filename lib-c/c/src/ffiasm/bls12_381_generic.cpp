#include "bls12_381.hpp"
#include <cstdint>
#include <cstring>
#include <cassert>

BLS12_381Element BLS12_381_q  = {0, 0x80000000, {0xffffffff00000001,0x53bda402fffe5bfe,0x3339d80809a1d805,0x73eda753299d7d48}};
BLS12_381Element BLS12_381_R2 = {0, 0x80000000, {0xc999e990f3f29c6d,0x2b6cedcb87925c23,0x05d314967254398f,0x0748d9d99f59ff11}};
BLS12_381Element BLS12_381_R3 = {0, 0x80000000, {0xc62c1807439b73af,0x1b3e0d188cf06990,0x73d13c71c7b5f418,0x6e2a5bb9c8db33e9}};

static BLS12_381RawElement half = {0x7fffffff80000000,0xa9ded2017fff2dff,0x199cec0404d0ec02,0x39f6d3a994cebea4};
static BLS12_381RawElement zero = {0};


void BLS12_381_copy(PBLS12_381Element r, const PBLS12_381Element a)
{
    *r = *a;
}

void BLS12_381_toNormal(PBLS12_381Element r, PBLS12_381Element a)
{
    if (a->type == BLS12_381_LONGMONTGOMERY)
    {
        r->type = BLS12_381_LONG;
        BLS12_381_rawFromMontgomery(r->longVal, a->longVal);
    }
    else
    {
        BLS12_381_copy(r, a);
    }
}

static inline int has_mul32_overflow(int64_t val)
{
    int64_t sign = val >> 31;

    if (sign)
    {
        sign = ~sign;
    }

    return sign ? 1 : 0;
}

static inline int BLS12_381_rawSMul(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a * b;

    return has_mul32_overflow(*r);
}

static inline void mul_s1s2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    int64_t result;

    int overflow = BLS12_381_rawSMul(&result, a->shortVal, b->shortVal);

    if (overflow)
    {
        BLS12_381_rawCopyS2L(r->longVal, result);
        r->type = BLS12_381_LONG;
        r->shortVal = 0;
    }
    else
    {
        // done the same way as in intel asm implementation
        r->shortVal = (int32_t)result;
        r->type = BLS12_381_SHORT;
        //

        BLS12_381_rawCopyS2L(r->longVal, result);
        r->type = BLS12_381_LONG;
        r->shortVal = 0;
    }
}

static inline void mul_l1nl2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381_rawMMul(r->longVal, a->longVal, b->longVal);
    BLS12_381_rawMMul(r->longVal, r->longVal, BLS12_381_R3.longVal);
}

static inline void mul_l1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;
    BLS12_381_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;
    BLS12_381_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;
    BLS12_381_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ns2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        BLS12_381_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        BLS12_381_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        BLS12_381_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }

    BLS12_381_rawMMul(r->longVal, r->longVal, BLS12_381_R3.longVal);
}

static inline void mul_s1nl2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        BLS12_381_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        BLS12_381_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        BLS12_381_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }

    BLS12_381_rawMMul(r->longVal, r->longVal, BLS12_381_R3.longVal);
}

static inline void mul_l1ms2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        BLS12_381_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        BLS12_381_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        BLS12_381_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void mul_s1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        BLS12_381_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        BLS12_381_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        BLS12_381_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }
}

static inline void mul_l1ns2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;
    BLS12_381_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ms2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;
    BLS12_381_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;
    BLS12_381_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;
    BLS12_381_rawMMul(r->longVal, a->longVal, b->longVal);
}

void BLS12_381_mul(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    mul_l1ml2m(r, a, b);
                }
                else
                {
                    mul_l1ml2n(r, a, b);
                }
            }
            else
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    mul_l1nl2m(r, a, b);
                }
                else
                {
                    mul_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            if (b->type & BLS12_381_MONTGOMERY)
            {
                mul_l1ms2m(r, a, b);
            }
            else
            {
                mul_l1ms2n(r, a, b);
            }
        }
        else
        {
            if (b->type & BLS12_381_MONTGOMERY)
            {
                mul_l1ns2m(r, a, b);
            }
            else
            {
                mul_l1ns2n(r, a, b);
            }
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (a->type & BLS12_381_MONTGOMERY)
        {
            if (b->type & BLS12_381_MONTGOMERY)
            {
                mul_s1ml2m(r, a, b);
            }
            else
            {
                mul_s1ml2n(r,a, b);
            }
        }
        else if (b->type & BLS12_381_MONTGOMERY)
        {
            mul_s1nl2m(r, a, b);
        }
        else
        {
            mul_s1nl2n(r, a, b);
        }
    }
    else
    {
         mul_s1s2(r, a, b);
    }
}

void BLS12_381_toLongNormal(PBLS12_381Element r, PBLS12_381Element a)
{
    if (a->type & BLS12_381_LONG)
    {
        if (a->type & BLS12_381_MONTGOMERY)
        {
            BLS12_381_rawFromMontgomery(r->longVal, a->longVal);
            r->type = BLS12_381_LONG;
        }
        else
        {
            BLS12_381_copy(r, a);
        }
    }
    else
    {
        BLS12_381_rawCopyS2L(r->longVal, a->shortVal);
        r->type = BLS12_381_LONG;
        r->shortVal = 0;
    }
}

void BLS12_381_toMontgomery(PBLS12_381Element r, PBLS12_381Element a)
{
    if (a->type & BLS12_381_MONTGOMERY)
    {
        BLS12_381_copy(r, a);
    }
    else if (a->type & BLS12_381_LONG)
    {
        r->shortVal = a->shortVal;

        BLS12_381_rawMMul(r->longVal, a->longVal, BLS12_381_R2.longVal);

        r->type = BLS12_381_LONGMONTGOMERY;
    }
    else if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
       BLS12_381_rawMMul1(r->longVal, BLS12_381_R2.longVal, -a_shortVal);
       BLS12_381_rawNeg(r->longVal, r->longVal);

       r->type = BLS12_381_SHORTMONTGOMERY;
    }
    else
    {
        BLS12_381_rawMMul1(r->longVal, BLS12_381_R2.longVal, a->shortVal);

        r->type = BLS12_381_SHORTMONTGOMERY;
    }
}

void BLS12_381_copyn(PBLS12_381Element r, PBLS12_381Element a, int n)
{
    std::memcpy(r, a, n * sizeof(BLS12_381Element));
}

static inline int has_add32_overflow(int64_t val)
{
    int64_t signs = (val >> 31) & 0x3;

    return signs == 1 || signs == 2;
}

static inline int BLS12_381_rawSSub(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a - b;

    return has_add32_overflow(*r);
}

static inline void sub_s1s2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    int64_t diff;

    int overflow = BLS12_381_rawSSub(&diff, a->shortVal, b->shortVal);

    if (overflow)
    {
        BLS12_381_rawCopyS2L(r->longVal, diff);
        r->type = BLS12_381_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = BLS12_381_SHORT;
        r->shortVal = (int32_t)diff;
    }
}

static inline void sub_l1nl2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381Element a_m;
    BLS12_381_toMontgomery(&a_m, a);

    BLS12_381_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1ml2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381Element b_m;
    BLS12_381_toMontgomery(&b_m, b);

    BLS12_381_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1l2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    if (a->shortVal >= 0)
    {
        BLS12_381_rawSubSL(r->longVal, a->shortVal, b->longVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        BLS12_381_rawNegLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void sub_l1ms2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381Element b_m;
    BLS12_381_toMontgomery(&b_m, b);

    BLS12_381_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381Element a_m;
    BLS12_381_toMontgomery(&a_m, a);

    BLS12_381_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ns2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        BLS12_381_rawAddLS(r->longVal, a->longVal, -b_shortVal);
    }
    else
    {
        BLS12_381_rawSubLS(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void sub_l1ms2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_s1ml2m(PBLS12_381Element r,PBLS12_381Element a,PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381_rawSub(r->longVal, a->longVal, b->longVal);
}

void BLS12_381_sub(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    sub_l1ml2m(r, a, b);
                }
                else
                {
                    sub_l1ml2n(r, a, b);
                }
            }
            else if (b->type & BLS12_381_MONTGOMERY)
            {
                sub_l1nl2m(r, a, b);
            }
            else
            {
                sub_l1nl2n(r, a, b);
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            if (b->type & BLS12_381_MONTGOMERY)
            {
                sub_l1ms2m(r, a, b);
            }
            else
            {
                sub_l1ms2n(r, a, b);
            }
        }
        else
        {
            sub_l1ns2(r, a, b);
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_MONTGOMERY)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
               sub_s1ml2m(r,a,b);
            }
            else
            {
               sub_s1nl2m(r,a,b);
            }
        }
        else
        {
            sub_s1l2n(r,a,b);
        }
    }
    else
    {
         sub_s1s2(r, a, b);
    }
}

static inline int BLS12_381_rawSAdd(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a + b;

    return has_add32_overflow(*r);
}

static inline void add_s1s2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    int64_t sum;

    int overflow = BLS12_381_rawSAdd(&sum, a->shortVal, b->shortVal);

    if (overflow)
    {
        BLS12_381_rawCopyS2L(r->longVal, sum);
        r->type = BLS12_381_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = BLS12_381_SHORT;
        r->shortVal = (int32_t)sum;
    }
}

static inline void add_l1nl2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381Element a_m;
    BLS12_381_toMontgomery(&a_m, a);

    BLS12_381_rawAdd(r->longVal, a_m.longVal, b->longVal);
}

static inline void add_l1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;
    BLS12_381_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1ml2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381Element b_m;
    BLS12_381_toMontgomery(&b_m, b);

    BLS12_381_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1l2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    if (a->shortVal >= 0)
    {
        BLS12_381_rawAddLS(r->longVal, b->longVal, a->shortVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        BLS12_381_rawSubLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void add_l1ms2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element b_m;

    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381_toMontgomery(&b_m, b);

    BLS12_381_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381Element m_a;
    BLS12_381_toMontgomery(&m_a, a);

    BLS12_381_rawAdd(r->longVal, m_a.longVal, b->longVal);
}

static inline void add_l1ns2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    if (b->shortVal >= 0)
    {
        BLS12_381_rawAddLS(r->longVal, a->longVal, b->shortVal);
    }
    else
    {
        int64_t b_shortVal = b->shortVal;
        BLS12_381_rawSubLS(r->longVal, a->longVal, -b_shortVal);
    }
}

static inline void add_l1ms2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_s1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONGMONTGOMERY;

    BLS12_381_rawAdd(r->longVal, a->longVal, b->longVal);
}

void BLS12_381_add(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    add_l1ml2m(r, a, b);
                }
                else
                {
                    add_l1ml2n(r, a, b);
                }
            }
            else
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    add_l1nl2m(r, a, b);
                }
                else
                {
                    add_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            if (b->type & BLS12_381_MONTGOMERY)
            {
                add_l1ms2m(r, a, b);
            }
            else
            {
                add_l1ms2n(r, a, b);
            }
        }
        else
        {
            add_l1ns2(r, a, b);
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_MONTGOMERY)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
               add_s1ml2m(r, a, b);
            }
            else
            {
               add_s1nl2m(r, a, b);
            }
        }
        else
        {
            add_s1l2n(r, a, b);
        }
    }
    else
    {
        add_s1s2(r, a, b);
    }
}

int BLS12_381_isTrue(PBLS12_381Element pE)
{
    int result;

    if (pE->type & BLS12_381_LONG)
    {
        result = !BLS12_381_rawIsZero(pE->longVal);
    }
    else
    {
        result = pE->shortVal != 0;
    }

    return result;
}

int BLS12_381_longNeg(PBLS12_381Element pE)
{
    if(BLS12_381_rawCmp(pE->longVal, BLS12_381_q.longVal) >= 0)
    {
       BLS12_381_longErr();
       return 0;
    }

    int64_t result = pE->longVal[0] - BLS12_381_q.longVal[0];

    int64_t is_long = (result >> 31) + 1;

    if(is_long)
    {
       BLS12_381_longErr();
       return 0;
    }

    return result;
}

int BLS12_381_longNormal(PBLS12_381Element pE)
{
    uint64_t is_long = 0;
    uint64_t result;

    result = pE->longVal[0];

    is_long = result >> 31;

    if (is_long)
    {
         return BLS12_381_longNeg(pE);
    }

    if (memcmp(&pE->longVal[1], zero, (sizeof(pE->longVal) - sizeof(pE->longVal[0]))))
    {
        return BLS12_381_longNeg(pE);
    }

    return result;
}

// Convert a 64 bit integer to a long format field element
int BLS12_381_toInt(PBLS12_381Element pE)
{
    int result;

    if (pE->type & BLS12_381_LONG)
    {
       if (pE->type & BLS12_381_MONTGOMERY)
       {
           BLS12_381Element e_n;
           BLS12_381_toNormal(&e_n, pE);

           result = BLS12_381_longNormal(&e_n);
       }
       else
       {
           result = BLS12_381_longNormal(pE);
       }
    }
    else
    {
        result = pE->shortVal;
    }

    return result;
}

static inline int rlt_s1s2(PBLS12_381Element a, PBLS12_381Element b)
{
    return (a->shortVal < b->shortVal) ? 1 : 0;
}

static inline int rltRawL1L2(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    int result = BLS12_381_rawCmp(pRawB, pRawA);

    return result > 0 ? 1 : 0;
}

static inline int rltl1l2_n1(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    int result = BLS12_381_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rltRawL1L2(pRawA, pRawB);
    }

     return 1;
}

static inline int rltl1l2_p1(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    int result = BLS12_381_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 0;
    }

    return rltRawL1L2(pRawA, pRawB);
}

static inline int rltL1L2(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    int result = BLS12_381_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rltl1l2_n1(pRawA, pRawB);
    }

    return rltl1l2_p1(pRawA, pRawB);
}

static inline int rlt_l1nl2n(PBLS12_381Element a, PBLS12_381Element b)
{
    return rltL1L2(a->longVal, b->longVal);
}

static inline int rlt_l1nl2m(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element b_n;

    BLS12_381_toNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

static inline int rlt_l1ml2m(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);
    BLS12_381_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ml2n(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;

    BLS12_381_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_s1l2n(PBLS12_381Element a,PBLS12_381Element b)
{
    BLS12_381Element a_n;

    BLS12_381_toLongNormal(&a_n,a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_l1ms2(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381Element b_ln;

    BLS12_381_toLongNormal(&b_ln ,b);
    BLS12_381_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b_ln.longVal);
}

static inline int rlt_s1l2m(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toLongNormal(&a_n, a);
    BLS12_381_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ns2(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element b_n;

    BLS12_381_toLongNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

int32_t BLS12_381_rlt(PBLS12_381Element a, PBLS12_381Element b)
{
    int32_t result;

    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    result = rlt_l1ml2m(a, b);
                }
                else
                {
                    result = rlt_l1ml2n(a, b);
                }
            }
            else if (b->type & BLS12_381_MONTGOMERY)
            {
                result = rlt_l1nl2m(a, b);
            }
            else
            {
                result = rlt_l1nl2n(a, b);
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            result = rlt_l1ms2(a, b);
        }
        else
        {
            result = rlt_l1ns2(a, b);
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_MONTGOMERY)
        {
            result = rlt_s1l2m(a,b);
        }
        else
        {
            result = rlt_s1l2n(a,b);
        }
    }
    else
    {
         result = rlt_s1s2(a, b);
    }

    return result;
}

void BLS12_381_lt(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->shortVal = BLS12_381_rlt(a, b);
    r->type = BLS12_381_SHORT;
}

void BLS12_381_geq(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
   int32_t result = BLS12_381_rlt(a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = BLS12_381_SHORT;
}

static inline int BLS12_381_rawSNeg(int64_t *r, int32_t a)
{
    *r = -(int64_t)a;

    return has_add32_overflow(*r);
}

void BLS12_381_neg(PBLS12_381Element r, PBLS12_381Element a)
{
    if (a->type & BLS12_381_LONG)
    {
        r->type = a->type;
        r->shortVal = a->shortVal;
        BLS12_381_rawNeg(r->longVal, a->longVal);
    }
    else
    {
        int64_t a_shortVal;

        int overflow = BLS12_381_rawSNeg(&a_shortVal, a->shortVal);

        if (overflow)
        {
            BLS12_381_rawCopyS2L(r->longVal, a_shortVal);
            r->type = BLS12_381_LONG;
            r->shortVal = 0;
        }
        else
        {
            r->type = BLS12_381_SHORT;
            r->shortVal = (int32_t)a_shortVal;
        }
    }
}

static inline int reqL1L2(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    return BLS12_381_rawCmp(pRawB, pRawA) == 0;
}

static inline int req_s1s2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    return (a->shortVal == b->shortVal) ? 1 : 0;
}

static inline int req_l1nl2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_m;
    BLS12_381_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1ml2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element b_m;
    BLS12_381_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381_toLongNormal(&a_n, a);

    return reqL1L2(a_n.longVal, b->longVal);
}

static inline int req_l1ms2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element b_m;
    BLS12_381_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_m;
    BLS12_381_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ns2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element b_n;
    BLS12_381_toLongNormal(&b_n, b);

    return reqL1L2(a->longVal, b_n.longVal);
}

// Compares two elements of any kind
int BLS12_381_req(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    int result;

    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    result = req_l1ml2m(r, a, b);
                }
                else
                {
                    result = req_l1ml2n(r, a, b);
                }
            }
            else if (b->type & BLS12_381_MONTGOMERY)
            {
                result = req_l1nl2m(r, a, b);
            }
            else
            {
                result = req_l1nl2n(r, a, b);
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            result = req_l1ms2(r, a, b);
        }
        else
        {
            result = req_l1ns2(r, a, b);
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_MONTGOMERY)
        {
            result = req_s1l2m(r, a, b);
        }
        else
        {
            result = req_s1l2n(r, a, b);
        }
    }
    else
    {
         result = req_s1s2(r, a, b);
    }

    return result;
}

void BLS12_381_eq(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->shortVal = BLS12_381_req(r, a, b);
    r->type = BLS12_381_SHORT;
}

void BLS12_381_neq(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    int result = BLS12_381_req(r, a, b);

    r->shortVal = result ^ 0x1;
    r->type = BLS12_381_SHORT;
}

// Logical or between two elements
void BLS12_381_lor(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    int32_t is_true_a;

    if (a->type & BLS12_381_LONG)
    {
        is_true_a = !BLS12_381_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & BLS12_381_LONG)
    {
        is_true_b = !BLS12_381_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a | is_true_b;
    r->type = BLS12_381_SHORT;
}

void BLS12_381_lnot(PBLS12_381Element r, PBLS12_381Element a)
{
    if (a->type & BLS12_381_LONG)
    {
        r->shortVal = BLS12_381_rawIsZero(a->longVal);
    }
    else
    {
        r->shortVal = a->shortVal ? 0 : 1;
    }

    r->type = BLS12_381_SHORT;
}


static inline int rgt_s1s2(PBLS12_381Element a, PBLS12_381Element b)
{
    return (a->shortVal > b->shortVal) ? 1 : 0;
}

static inline int rgtRawL1L2(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    int result = BLS12_381_rawCmp(pRawB, pRawA);

    return (result < 0) ? 1 : 0;
}

static inline int rgtl1l2_n1(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    int result = BLS12_381_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rgtRawL1L2(pRawA, pRawB);
    }
    return 0;
}

static inline int rgtl1l2_p1(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    int result = BLS12_381_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 1;
    }
    return rgtRawL1L2(pRawA, pRawB);
}

static inline int rgtL1L2(BLS12_381RawElement pRawA, BLS12_381RawElement pRawB)
{
    int result = BLS12_381_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rgtl1l2_n1(pRawA, pRawB);
    }

    return rgtl1l2_p1(pRawA, pRawB);
}

static inline int rgt_l1nl2n(PBLS12_381Element a, PBLS12_381Element b)
{
    return rgtL1L2(a->longVal, b->longVal);
}

static inline int rgt_l1nl2m(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element b_n;
    BLS12_381_toNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

static inline int rgt_l1ml2m(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);
    BLS12_381_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ml2n(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381_toNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_s1l2n(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381_toLongNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_l1ms2(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);
    BLS12_381_toLongNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_s1l2m(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toLongNormal(&a_n, a);
    BLS12_381_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ns2(PBLS12_381Element a, PBLS12_381Element b)
{
    BLS12_381Element b_n;
    BLS12_381_toLongNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

int BLS12_381_rgt(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    int result = 0;

    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    result = rgt_l1ml2m(a, b);
                }
                else
                {
                    result = rgt_l1ml2n(a, b);
                }
            }
            else if (b->type & BLS12_381_MONTGOMERY)
            {
                result = rgt_l1nl2m(a, b);
            }
            else
            {
                result = rgt_l1nl2n(a, b);
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            result = rgt_l1ms2(a, b);
        }
        else
        {
            result = rgt_l1ns2(a, b);
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_MONTGOMERY)
        {
            result = rgt_s1l2m(a, b);
        }
        else
        {
            result = rgt_s1l2n(a,b);
        }
    }
    else
    {
         result = rgt_s1s2(a, b);
    }

    return result;
}

void BLS12_381_gt(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->shortVal = BLS12_381_rgt(r, a, b);
    r->type = BLS12_381_SHORT;
}

void BLS12_381_leq(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
   int32_t result = BLS12_381_rgt(r, a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = BLS12_381_SHORT;
}

// Logical and between two elements
void BLS12_381_land(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    int32_t is_true_a;

    if (a->type & BLS12_381_LONG)
    {
        is_true_a = !BLS12_381_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & BLS12_381_LONG)
    {
        is_true_b = !BLS12_381_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a & is_true_b;
    r->type = BLS12_381_SHORT;
}

static inline void and_s1s2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        int32_t result = a->shortVal & b->shortVal;
        r->shortVal = result;
        r->type = BLS12_381_SHORT;
        return;
    }

    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toLongNormal(&a_n, a);
    BLS12_381_toLongNormal(&b_n, b);

    BLS12_381_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1nl2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;
    BLS12_381_rawAnd(r->longVal, a->longVal, b->longVal);
}

static inline void and_l1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element b_n;
    BLS12_381_toNormal(&b_n, b);

    BLS12_381_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

static inline void and_l1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);
    BLS12_381_toNormal(&b_n, b);

    BLS12_381_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ml2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381_toNormal(&a_n, a);

    BLS12_381_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_s1l2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&a_n, a);
    }

    BLS12_381_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_l1ms2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&b_n, b);
    }

    BLS12_381_rawAnd(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void and_s1l2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&a_n, a);
    }

    BLS12_381_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ns2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&b_n, b);
    }

    BLS12_381_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

// Ands two elements of any kind
void BLS12_381_band(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    and_l1ml2m(r, a, b);
                }
                else
                {
                    and_l1ml2n(r, a, b);
                }
            }
            else if (b->type & BLS12_381_MONTGOMERY)
            {
                and_l1nl2m(r, a, b);
            }
            else
            {
                and_l1nl2n(r, a, b);
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            and_l1ms2(r, a, b);
        }
        else
        {
           and_l1ns2(r, a, b);
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_MONTGOMERY)
        {
            and_s1l2m(r, a, b);
        }
        else
        {
            and_s1l2n(r, a, b);
        }
    }
    else
    {
         and_s1s2(r, a, b);
    }
}

void BLS12_381_rawZero(BLS12_381RawElement pRawResult)
{
    std::memset(pRawResult, 0, sizeof(BLS12_381RawElement));
}

static inline void rawShl(BLS12_381RawElement r, BLS12_381RawElement a, uint64_t b)
{
    if (b == 0)
    {
        BLS12_381_rawCopy(r, a);
        return;
    }

    if (b >= 255)
    {
        BLS12_381_rawZero(r);
        return;
    }

    BLS12_381_rawShl(r, a, b);
}

static inline void rawShr(BLS12_381RawElement r, BLS12_381RawElement a, uint64_t b)
{
    if (b == 0)
    {
        BLS12_381_rawCopy(r, a);
        return;
    }

    if (b >= 255)
    {
        BLS12_381_rawZero(r);
        return;
    }

    BLS12_381_rawShr(r,a, b);
}

static inline void BLS12_381_setzero(PBLS12_381Element r)
{
    r->type = 0;
    r->shortVal = 0;
}

static inline void do_shlcl(PBLS12_381Element r, PBLS12_381Element a, uint64_t b)
{
    BLS12_381Element a_long;
    BLS12_381_toLongNormal(&a_long, a);

    r->type = BLS12_381_LONG;
    rawShl(r->longVal, a_long.longVal, b);
}

static inline void do_shlln(PBLS12_381Element r, PBLS12_381Element a, uint64_t b)
{
    r->type = BLS12_381_LONG;
    rawShl(r->longVal, a->longVal, b);
}

static inline void do_shl(PBLS12_381Element r, PBLS12_381Element a, uint64_t b)
{
    if (a->type & BLS12_381_LONG)
    {
        if (a->type == BLS12_381_LONGMONTGOMERY)
        {
            BLS12_381Element a_long;
            BLS12_381_toNormal(&a_long, a);

            do_shlln(r, &a_long, b);
        }
        else
        {
            do_shlln(r, a, b);
        }
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            BLS12_381_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            do_shlcl(r, a, b);
        }
        else if(b >= 31)
        {
            do_shlcl(r, a, b);
        }
        else
        {
            a_shortVal <<= b;

            const uint64_t a_is_over_short = a_shortVal >> 31;

            if (a_is_over_short)
            {
                do_shlcl(r, a, b);
            }
            else
            {
                r->type = BLS12_381_SHORT;
                r->shortVal = a_shortVal;
            }
        }
    }
}

static inline void do_shrln(PBLS12_381Element r, PBLS12_381Element a, uint64_t b)
{
    r->type = BLS12_381_LONG;
    rawShr(r->longVal, a->longVal, b);
}

static inline void do_shrl(PBLS12_381Element r, PBLS12_381Element a, uint64_t b)
{
    if (a->type == BLS12_381_LONGMONTGOMERY)
    {
        BLS12_381Element a_long;
        BLS12_381_toNormal(&a_long, a);

        do_shrln(r, &a_long, b);
    }
    else
    {
        do_shrln(r, a, b);
    }
}

static inline void do_shr(PBLS12_381Element r, PBLS12_381Element a, uint64_t b)
{
    if (a->type & BLS12_381_LONG)
    {
        do_shrl(r, a, b);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            BLS12_381_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            BLS12_381Element a_long;
            BLS12_381_toLongNormal(&a_long, a);

            do_shrl(r, &a_long, b);
        }
        else if(b >= 31)
        {
            BLS12_381_setzero(r);
        }
        else
        {
            a_shortVal >>= b;

            r->shortVal = a_shortVal;
            r->type = BLS12_381_SHORT;
        }
    }
}

static inline void BLS12_381_shr_big_shift(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    static BLS12_381RawElement max_shift = {255};

    BLS12_381RawElement shift;

    BLS12_381_rawSubRegular(shift, BLS12_381_q.longVal, b->longVal);

    if (BLS12_381_rawCmp(shift, max_shift) >= 0)
    {
        BLS12_381_setzero(r);
    }
    else
    {
        do_shl(r, a, shift[0]);
    }
}

static inline void BLS12_381_shr_long(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    static BLS12_381RawElement max_shift = {255};

    if (BLS12_381_rawCmp(b->longVal, max_shift) >= 0)
    {
        BLS12_381_shr_big_shift(r, a, b);
    }
    else
    {
        do_shr(r, a, b->longVal[0]);
    }
}

void BLS12_381_shr(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (b->type & BLS12_381_LONG)
    {
        if (b->type == BLS12_381_LONGMONTGOMERY)
        {
            BLS12_381Element b_long;
            BLS12_381_toNormal(&b_long, b);

            BLS12_381_shr_long(r, a, &b_long);
        }
        else
        {
            BLS12_381_shr_long(r, a, b);
        }
    }
    else
    {
        int64_t b_shortVal = b->shortVal;

        if (b_shortVal < 0)
        {
            b_shortVal = -b_shortVal;

            if (b_shortVal >= 255)
            {
                BLS12_381_setzero(r);
            }
            else
            {
                do_shl(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 255)
        {
            BLS12_381_setzero(r);
        }
        else
        {
            do_shr(r, a, b_shortVal);
        }
    }
}

static inline void BLS12_381_shl_big_shift(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    static BLS12_381RawElement max_shift = {255};

    BLS12_381RawElement shift;

    BLS12_381_rawSubRegular(shift, BLS12_381_q.longVal, b->longVal);

    if (BLS12_381_rawCmp(shift, max_shift) >= 0)
    {
        BLS12_381_setzero(r);
    }
    else
    {
        do_shr(r, a, shift[0]);
    }
}

static inline void BLS12_381_shl_long(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    static BLS12_381RawElement max_shift = {255};

    if (BLS12_381_rawCmp(b->longVal, max_shift) >= 0)
    {
        BLS12_381_shl_big_shift(r, a, b);
    }
    else
    {
        do_shl(r, a, b->longVal[0]);
    }
}

void BLS12_381_shl(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (b->type & BLS12_381_LONG)
    {
        if (b->type == BLS12_381_LONGMONTGOMERY)
        {
            BLS12_381Element b_long;
            BLS12_381_toNormal(&b_long, b);

            BLS12_381_shl_long(r, a, &b_long);
        }
        else
        {
            BLS12_381_shl_long(r, a, b);
        }
    }
    else
    {
        int64_t b_shortVal = b->shortVal;

        if (b_shortVal < 0)
        {
            b_shortVal = -b_shortVal;

            if (b_shortVal >= 255)
            {
                BLS12_381_setzero(r);
            }
            else
            {
                do_shr(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 255)
        {
            BLS12_381_setzero(r);
        }
        else
        {
            do_shl(r, a, b_shortVal);
        }
    }
}

void BLS12_381_square(PBLS12_381Element r, PBLS12_381Element a)
{
    if (a->type & BLS12_381_LONG)
    {
        if (a->type == BLS12_381_LONGMONTGOMERY)
        {
            r->type = BLS12_381_LONGMONTGOMERY;
            BLS12_381_rawMSquare(r->longVal, a->longVal);
        }
        else
        {
            r->type = BLS12_381_LONGMONTGOMERY;
            BLS12_381_rawMSquare(r->longVal, a->longVal);
            BLS12_381_rawMMul(r->longVal, r->longVal, BLS12_381_R3.longVal);
        }
    }
    else
    {
        int64_t result;

        int overflow = BLS12_381_rawSMul(&result, a->shortVal, a->shortVal);

        if (overflow)
        {
            BLS12_381_rawCopyS2L(r->longVal, result);
            r->type = BLS12_381_LONG;
            r->shortVal = 0;
        }
        else
        {
            // done the same way as in intel asm implementation
            r->shortVal = (int32_t)result;
            r->type = BLS12_381_SHORT;
            //

            BLS12_381_rawCopyS2L(r->longVal, result);
            r->type = BLS12_381_LONG;
            r->shortVal = 0;
        }
    }
}

static inline void or_s1s2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal | b->shortVal;
        r->type = BLS12_381_SHORT;
        return;
    }

    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toLongNormal(&a_n, a);
    BLS12_381_toLongNormal(&b_n, b);

    BLS12_381_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&a_n, a);
    }

    BLS12_381_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&a_n, a);
    }

    BLS12_381_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ns2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&b_n, b);
    }

    BLS12_381_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ms2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&b_n, b);
    }

    BLS12_381_rawOr(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void or_l1nl2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;
    BLS12_381_rawOr(r->longVal, a->longVal, b->longVal);
}

static inline void or_l1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element b_n;
    BLS12_381_toNormal(&b_n, b);

    BLS12_381_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ml2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381_toNormal(&a_n, a);

    BLS12_381_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);
    BLS12_381_toNormal(&b_n, b);

    BLS12_381_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}


void BLS12_381_bor(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    or_l1ml2m(r, a, b);
                }
                else
                {
                    or_l1ml2n(r, a, b);
                }
            }
            else if (b->type & BLS12_381_MONTGOMERY)
            {
                or_l1nl2m(r, a, b);
            }
            else
            {
                or_l1nl2n(r, a, b);
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            or_l1ms2(r, a, b);
        }
        else
        {
           or_l1ns2(r, a, b);
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_MONTGOMERY)
        {
            or_s1l2m(r, a, b);
        }
        else
        {
            or_s1l2n(r, a, b);
        }
    }
    else
    {
         or_s1s2(r, a, b);
    }
}

static inline void xor_s1s2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal ^ b->shortVal;
        r->type = BLS12_381_SHORT;
        return;
    }

    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toLongNormal(&a_n, a);
    BLS12_381_toLongNormal(&b_n, b);

    BLS12_381_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_s1l2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&a_n, a);
    }

    BLS12_381_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_s1l2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&a_n, a);
    }

    BLS12_381_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_l1ns2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&b_n, b);
    }

    BLS12_381_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ms2(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        BLS12_381_toLongNormal(&b_n, b);
    }

    BLS12_381_rawXor(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void xor_l1nl2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;
    BLS12_381_rawXor(r->longVal, a->longVal, b->longVal);
}

static inline void xor_l1nl2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element b_n;
    BLS12_381_toNormal(&b_n, b);

    BLS12_381_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ml2n(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381_toNormal(&a_n, a);

    BLS12_381_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_l1ml2m(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    r->type = BLS12_381_LONG;

    BLS12_381Element a_n;
    BLS12_381Element b_n;

    BLS12_381_toNormal(&a_n, a);
    BLS12_381_toNormal(&b_n, b);

    BLS12_381_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

void BLS12_381_bxor(PBLS12_381Element r, PBLS12_381Element a, PBLS12_381Element b)
{
    if (a->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_LONG)
        {
            if (a->type & BLS12_381_MONTGOMERY)
            {
                if (b->type & BLS12_381_MONTGOMERY)
                {
                    xor_l1ml2m(r, a, b);
                }
                else
                {
                    xor_l1ml2n(r, a, b);
                }
            }
            else if (b->type & BLS12_381_MONTGOMERY)
            {
                xor_l1nl2m(r, a, b);
            }
            else
            {
                xor_l1nl2n(r, a, b);
            }
        }
        else if (a->type & BLS12_381_MONTGOMERY)
        {
            xor_l1ms2(r, a, b);
        }
        else
        {
           xor_l1ns2(r, a, b);
        }
    }
    else if (b->type & BLS12_381_LONG)
    {
        if (b->type & BLS12_381_MONTGOMERY)
        {
            xor_s1l2m(r, a, b);
        }
        else
        {
            xor_s1l2n(r, a, b);
        }
    }
    else
    {
         xor_s1s2(r, a, b);
    }
}

void BLS12_381_bnot(PBLS12_381Element r, PBLS12_381Element a)
{
    r->type = BLS12_381_LONG;

    if (a->type == BLS12_381_LONG)
    {
        if (a->type & BLS12_381_MONTGOMERY)
        {
            BLS12_381Element a_n;
            BLS12_381_toNormal(&a_n, a);

            BLS12_381_rawNot(r->longVal, a_n.longVal);
        }
        else
        {
            BLS12_381_rawNot(r->longVal, a->longVal);
        }
    }
    else
    {
        BLS12_381Element a_n;
        BLS12_381_toLongNormal(&a_n, a);

        BLS12_381_rawNot(r->longVal, a_n.longVal);
    }
}
