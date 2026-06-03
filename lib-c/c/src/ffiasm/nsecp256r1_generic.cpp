#include "nsecp256r1.hpp"
#include <cstdint>
#include <cstring>
#include <cassert>

nSecp256r1Element nSecp256r1_q  = {0, 0x80000000, {0xf3b9cac2fc632551,0xbce6faada7179e84,0xffffffffffffffff,0xffffffff00000000}};
nSecp256r1Element nSecp256r1_R2 = {0, 0x80000000, {0x83244c95be79eea2,0x4699799c49bd6fa6,0x2845b2392b6bec59,0x66e12d94f3d95620}};
nSecp256r1Element nSecp256r1_R3 = {0, 0x80000000, {0xac8ebec90b65a624,0x111f28ae0c0555c9,0x2543b9246ba5e93f,0x503a54e76407be65}};

static nSecp256r1RawElement half = {0x79dce5617e3192a8,0xde737d56d38bcf42,0x7fffffffffffffff,0x7fffffff80000000};
static nSecp256r1RawElement zero = {0};


void nSecp256r1_copy(PnSecp256r1Element r, const PnSecp256r1Element a)
{
    *r = *a;
}

void nSecp256r1_toNormal(PnSecp256r1Element r, PnSecp256r1Element a)
{
    if (a->type == nSecp256r1_LONGMONTGOMERY)
    {
        r->type = nSecp256r1_LONG;
        nSecp256r1_rawFromMontgomery(r->longVal, a->longVal);
    }
    else
    {
        nSecp256r1_copy(r, a);
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

static inline int nSecp256r1_rawSMul(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a * b;

    return has_mul32_overflow(*r);
}

static inline void mul_s1s2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    int64_t result;

    int overflow = nSecp256r1_rawSMul(&result, a->shortVal, b->shortVal);

    if (overflow)
    {
        nSecp256r1_rawCopyS2L(r->longVal, result);
        r->type = nSecp256r1_LONG;
        r->shortVal = 0;
    }
    else
    {
        // done the same way as in intel asm implementation
        r->shortVal = (int32_t)result;
        r->type = nSecp256r1_SHORT;
        //

        nSecp256r1_rawCopyS2L(r->longVal, result);
        r->type = nSecp256r1_LONG;
        r->shortVal = 0;
    }
}

static inline void mul_l1nl2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
    nSecp256r1_rawMMul(r->longVal, r->longVal, nSecp256r1_R3.longVal);
}

static inline void mul_l1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;
    nSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;
    nSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;
    nSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ns2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        nSecp256r1_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        nSecp256r1_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        nSecp256r1_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }

    nSecp256r1_rawMMul(r->longVal, r->longVal, nSecp256r1_R3.longVal);
}

static inline void mul_s1nl2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        nSecp256r1_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        nSecp256r1_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        nSecp256r1_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }

    nSecp256r1_rawMMul(r->longVal, r->longVal, nSecp256r1_R3.longVal);
}

static inline void mul_l1ms2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        nSecp256r1_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        nSecp256r1_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        nSecp256r1_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void mul_s1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        nSecp256r1_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        nSecp256r1_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        nSecp256r1_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }
}

static inline void mul_l1ns2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;
    nSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ms2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;
    nSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;
    nSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;
    nSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

void nSecp256r1_mul(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
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
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    mul_l1nl2m(r, a, b);
                }
                else
                {
                    mul_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            if (b->type & nSecp256r1_MONTGOMERY)
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
            if (b->type & nSecp256r1_MONTGOMERY)
            {
                mul_l1ns2m(r, a, b);
            }
            else
            {
                mul_l1ns2n(r, a, b);
            }
        }
    }
    else if (b->type & nSecp256r1_LONG)
    {
        if (a->type & nSecp256r1_MONTGOMERY)
        {
            if (b->type & nSecp256r1_MONTGOMERY)
            {
                mul_s1ml2m(r, a, b);
            }
            else
            {
                mul_s1ml2n(r,a, b);
            }
        }
        else if (b->type & nSecp256r1_MONTGOMERY)
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

void nSecp256r1_toLongNormal(PnSecp256r1Element r, PnSecp256r1Element a)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (a->type & nSecp256r1_MONTGOMERY)
        {
            nSecp256r1_rawFromMontgomery(r->longVal, a->longVal);
            r->type = nSecp256r1_LONG;
        }
        else
        {
            nSecp256r1_copy(r, a);
        }
    }
    else
    {
        nSecp256r1_rawCopyS2L(r->longVal, a->shortVal);
        r->type = nSecp256r1_LONG;
        r->shortVal = 0;
    }
}

void nSecp256r1_toMontgomery(PnSecp256r1Element r, PnSecp256r1Element a)
{
    if (a->type & nSecp256r1_MONTGOMERY)
    {
        nSecp256r1_copy(r, a);
    }
    else if (a->type & nSecp256r1_LONG)
    {
        r->shortVal = a->shortVal;

        nSecp256r1_rawMMul(r->longVal, a->longVal, nSecp256r1_R2.longVal);

        r->type = nSecp256r1_LONGMONTGOMERY;
    }
    else if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
       nSecp256r1_rawMMul1(r->longVal, nSecp256r1_R2.longVal, -a_shortVal);
       nSecp256r1_rawNeg(r->longVal, r->longVal);

       r->type = nSecp256r1_SHORTMONTGOMERY;
    }
    else
    {
        nSecp256r1_rawMMul1(r->longVal, nSecp256r1_R2.longVal, a->shortVal);

        r->type = nSecp256r1_SHORTMONTGOMERY;
    }
}

void nSecp256r1_copyn(PnSecp256r1Element r, PnSecp256r1Element a, int n)
{
    std::memcpy(r, a, n * sizeof(nSecp256r1Element));
}

static inline int has_add32_overflow(int64_t val)
{
    int64_t signs = (val >> 31) & 0x3;

    return signs == 1 || signs == 2;
}

static inline int nSecp256r1_rawSSub(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a - b;

    return has_add32_overflow(*r);
}

static inline void sub_s1s2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    int64_t diff;

    int overflow = nSecp256r1_rawSSub(&diff, a->shortVal, b->shortVal);

    if (overflow)
    {
        nSecp256r1_rawCopyS2L(r->longVal, diff);
        r->type = nSecp256r1_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = nSecp256r1_SHORT;
        r->shortVal = (int32_t)diff;
    }
}

static inline void sub_l1nl2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1Element a_m;
    nSecp256r1_toMontgomery(&a_m, a);

    nSecp256r1_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1ml2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1Element b_m;
    nSecp256r1_toMontgomery(&b_m, b);

    nSecp256r1_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1l2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    if (a->shortVal >= 0)
    {
        nSecp256r1_rawSubSL(r->longVal, a->shortVal, b->longVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        nSecp256r1_rawNegLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void sub_l1ms2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1Element b_m;
    nSecp256r1_toMontgomery(&b_m, b);

    nSecp256r1_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1Element a_m;
    nSecp256r1_toMontgomery(&a_m, a);

    nSecp256r1_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ns2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        nSecp256r1_rawAddLS(r->longVal, a->longVal, -b_shortVal);
    }
    else
    {
        nSecp256r1_rawSubLS(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void sub_l1ms2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_s1ml2m(PnSecp256r1Element r,PnSecp256r1Element a,PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1_rawSub(r->longVal, a->longVal, b->longVal);
}

void nSecp256r1_sub(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    sub_l1ml2m(r, a, b);
                }
                else
                {
                    sub_l1ml2n(r, a, b);
                }
            }
            else if (b->type & nSecp256r1_MONTGOMERY)
            {
                sub_l1nl2m(r, a, b);
            }
            else
            {
                sub_l1nl2n(r, a, b);
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            if (b->type & nSecp256r1_MONTGOMERY)
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
    else if (b->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_MONTGOMERY)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
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

static inline int nSecp256r1_rawSAdd(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a + b;

    return has_add32_overflow(*r);
}

static inline void add_s1s2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    int64_t sum;

    int overflow = nSecp256r1_rawSAdd(&sum, a->shortVal, b->shortVal);

    if (overflow)
    {
        nSecp256r1_rawCopyS2L(r->longVal, sum);
        r->type = nSecp256r1_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = nSecp256r1_SHORT;
        r->shortVal = (int32_t)sum;
    }
}

static inline void add_l1nl2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1Element a_m;
    nSecp256r1_toMontgomery(&a_m, a);

    nSecp256r1_rawAdd(r->longVal, a_m.longVal, b->longVal);
}

static inline void add_l1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;
    nSecp256r1_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1ml2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1Element b_m;
    nSecp256r1_toMontgomery(&b_m, b);

    nSecp256r1_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1l2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    if (a->shortVal >= 0)
    {
        nSecp256r1_rawAddLS(r->longVal, b->longVal, a->shortVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        nSecp256r1_rawSubLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void add_l1ms2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element b_m;

    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1_toMontgomery(&b_m, b);

    nSecp256r1_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1Element m_a;
    nSecp256r1_toMontgomery(&m_a, a);

    nSecp256r1_rawAdd(r->longVal, m_a.longVal, b->longVal);
}

static inline void add_l1ns2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    if (b->shortVal >= 0)
    {
        nSecp256r1_rawAddLS(r->longVal, a->longVal, b->shortVal);
    }
    else
    {
        int64_t b_shortVal = b->shortVal;
        nSecp256r1_rawSubLS(r->longVal, a->longVal, -b_shortVal);
    }
}

static inline void add_l1ms2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_s1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONGMONTGOMERY;

    nSecp256r1_rawAdd(r->longVal, a->longVal, b->longVal);
}

void nSecp256r1_add(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
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
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    add_l1nl2m(r, a, b);
                }
                else
                {
                    add_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            if (b->type & nSecp256r1_MONTGOMERY)
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
    else if (b->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_MONTGOMERY)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
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

int nSecp256r1_isTrue(PnSecp256r1Element pE)
{
    int result;

    if (pE->type & nSecp256r1_LONG)
    {
        result = !nSecp256r1_rawIsZero(pE->longVal);
    }
    else
    {
        result = pE->shortVal != 0;
    }

    return result;
}

int nSecp256r1_longNeg(PnSecp256r1Element pE)
{
    if(nSecp256r1_rawCmp(pE->longVal, nSecp256r1_q.longVal) >= 0)
    {
       nSecp256r1_longErr();
       return 0;
    }

    int64_t result = pE->longVal[0] - nSecp256r1_q.longVal[0];

    int64_t is_long = (result >> 31) + 1;

    if(is_long)
    {
       nSecp256r1_longErr();
       return 0;
    }

    return result;
}

int nSecp256r1_longNormal(PnSecp256r1Element pE)
{
    uint64_t is_long = 0;
    uint64_t result;

    result = pE->longVal[0];

    is_long = result >> 31;

    if (is_long)
    {
         return nSecp256r1_longNeg(pE);
    }

    if (memcmp(&pE->longVal[1], zero, (sizeof(pE->longVal) - sizeof(pE->longVal[0]))))
    {
        return nSecp256r1_longNeg(pE);
    }

    return result;
}

// Convert a 64 bit integer to a long format field element
int nSecp256r1_toInt(PnSecp256r1Element pE)
{
    int result;

    if (pE->type & nSecp256r1_LONG)
    {
       if (pE->type & nSecp256r1_MONTGOMERY)
       {
           nSecp256r1Element e_n;
           nSecp256r1_toNormal(&e_n, pE);

           result = nSecp256r1_longNormal(&e_n);
       }
       else
       {
           result = nSecp256r1_longNormal(pE);
       }
    }
    else
    {
        result = pE->shortVal;
    }

    return result;
}

static inline int rlt_s1s2(PnSecp256r1Element a, PnSecp256r1Element b)
{
    return (a->shortVal < b->shortVal) ? 1 : 0;
}

static inline int rltRawL1L2(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    int result = nSecp256r1_rawCmp(pRawB, pRawA);

    return result > 0 ? 1 : 0;
}

static inline int rltl1l2_n1(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    int result = nSecp256r1_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rltRawL1L2(pRawA, pRawB);
    }

     return 1;
}

static inline int rltl1l2_p1(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    int result = nSecp256r1_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 0;
    }

    return rltRawL1L2(pRawA, pRawB);
}

static inline int rltL1L2(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    int result = nSecp256r1_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rltl1l2_n1(pRawA, pRawB);
    }

    return rltl1l2_p1(pRawA, pRawB);
}

static inline int rlt_l1nl2n(PnSecp256r1Element a, PnSecp256r1Element b)
{
    return rltL1L2(a->longVal, b->longVal);
}

static inline int rlt_l1nl2m(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

static inline int rlt_l1ml2m(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);
    nSecp256r1_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ml2n(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;

    nSecp256r1_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_s1l2n(PnSecp256r1Element a,PnSecp256r1Element b)
{
    nSecp256r1Element a_n;

    nSecp256r1_toLongNormal(&a_n,a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_l1ms2(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1Element b_ln;

    nSecp256r1_toLongNormal(&b_ln ,b);
    nSecp256r1_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b_ln.longVal);
}

static inline int rlt_s1l2m(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toLongNormal(&a_n, a);
    nSecp256r1_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ns2(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element b_n;

    nSecp256r1_toLongNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

int32_t nSecp256r1_rlt(PnSecp256r1Element a, PnSecp256r1Element b)
{
    int32_t result;

    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    result = rlt_l1ml2m(a, b);
                }
                else
                {
                    result = rlt_l1ml2n(a, b);
                }
            }
            else if (b->type & nSecp256r1_MONTGOMERY)
            {
                result = rlt_l1nl2m(a, b);
            }
            else
            {
                result = rlt_l1nl2n(a, b);
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            result = rlt_l1ms2(a, b);
        }
        else
        {
            result = rlt_l1ns2(a, b);
        }
    }
    else if (b->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_MONTGOMERY)
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

void nSecp256r1_lt(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->shortVal = nSecp256r1_rlt(a, b);
    r->type = nSecp256r1_SHORT;
}

void nSecp256r1_geq(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
   int32_t result = nSecp256r1_rlt(a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = nSecp256r1_SHORT;
}

static inline int nSecp256r1_rawSNeg(int64_t *r, int32_t a)
{
    *r = -(int64_t)a;

    return has_add32_overflow(*r);
}

void nSecp256r1_neg(PnSecp256r1Element r, PnSecp256r1Element a)
{
    if (a->type & nSecp256r1_LONG)
    {
        r->type = a->type;
        r->shortVal = a->shortVal;
        nSecp256r1_rawNeg(r->longVal, a->longVal);
    }
    else
    {
        int64_t a_shortVal;

        int overflow = nSecp256r1_rawSNeg(&a_shortVal, a->shortVal);

        if (overflow)
        {
            nSecp256r1_rawCopyS2L(r->longVal, a_shortVal);
            r->type = nSecp256r1_LONG;
            r->shortVal = 0;
        }
        else
        {
            r->type = nSecp256r1_SHORT;
            r->shortVal = (int32_t)a_shortVal;
        }
    }
}

static inline int reqL1L2(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    return nSecp256r1_rawCmp(pRawB, pRawA) == 0;
}

static inline int req_s1s2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    return (a->shortVal == b->shortVal) ? 1 : 0;
}

static inline int req_l1nl2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_m;
    nSecp256r1_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1ml2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element b_m;
    nSecp256r1_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1_toLongNormal(&a_n, a);

    return reqL1L2(a_n.longVal, b->longVal);
}

static inline int req_l1ms2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element b_m;
    nSecp256r1_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_m;
    nSecp256r1_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ns2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element b_n;
    nSecp256r1_toLongNormal(&b_n, b);

    return reqL1L2(a->longVal, b_n.longVal);
}

// Compares two elements of any kind
int nSecp256r1_req(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    int result;

    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    result = req_l1ml2m(r, a, b);
                }
                else
                {
                    result = req_l1ml2n(r, a, b);
                }
            }
            else if (b->type & nSecp256r1_MONTGOMERY)
            {
                result = req_l1nl2m(r, a, b);
            }
            else
            {
                result = req_l1nl2n(r, a, b);
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            result = req_l1ms2(r, a, b);
        }
        else
        {
            result = req_l1ns2(r, a, b);
        }
    }
    else if (b->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_MONTGOMERY)
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

void nSecp256r1_eq(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->shortVal = nSecp256r1_req(r, a, b);
    r->type = nSecp256r1_SHORT;
}

void nSecp256r1_neq(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    int result = nSecp256r1_req(r, a, b);

    r->shortVal = result ^ 0x1;
    r->type = nSecp256r1_SHORT;
}

// Logical or between two elements
void nSecp256r1_lor(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    int32_t is_true_a;

    if (a->type & nSecp256r1_LONG)
    {
        is_true_a = !nSecp256r1_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & nSecp256r1_LONG)
    {
        is_true_b = !nSecp256r1_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a | is_true_b;
    r->type = nSecp256r1_SHORT;
}

void nSecp256r1_lnot(PnSecp256r1Element r, PnSecp256r1Element a)
{
    if (a->type & nSecp256r1_LONG)
    {
        r->shortVal = nSecp256r1_rawIsZero(a->longVal);
    }
    else
    {
        r->shortVal = a->shortVal ? 0 : 1;
    }

    r->type = nSecp256r1_SHORT;
}


static inline int rgt_s1s2(PnSecp256r1Element a, PnSecp256r1Element b)
{
    return (a->shortVal > b->shortVal) ? 1 : 0;
}

static inline int rgtRawL1L2(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    int result = nSecp256r1_rawCmp(pRawB, pRawA);

    return (result < 0) ? 1 : 0;
}

static inline int rgtl1l2_n1(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    int result = nSecp256r1_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rgtRawL1L2(pRawA, pRawB);
    }
    return 0;
}

static inline int rgtl1l2_p1(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    int result = nSecp256r1_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 1;
    }
    return rgtRawL1L2(pRawA, pRawB);
}

static inline int rgtL1L2(nSecp256r1RawElement pRawA, nSecp256r1RawElement pRawB)
{
    int result = nSecp256r1_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rgtl1l2_n1(pRawA, pRawB);
    }

    return rgtl1l2_p1(pRawA, pRawB);
}

static inline int rgt_l1nl2n(PnSecp256r1Element a, PnSecp256r1Element b)
{
    return rgtL1L2(a->longVal, b->longVal);
}

static inline int rgt_l1nl2m(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element b_n;
    nSecp256r1_toNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

static inline int rgt_l1ml2m(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);
    nSecp256r1_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ml2n(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1_toNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_s1l2n(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1_toLongNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_l1ms2(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);
    nSecp256r1_toLongNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_s1l2m(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toLongNormal(&a_n, a);
    nSecp256r1_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ns2(PnSecp256r1Element a, PnSecp256r1Element b)
{
    nSecp256r1Element b_n;
    nSecp256r1_toLongNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

int nSecp256r1_rgt(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    int result = 0;

    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    result = rgt_l1ml2m(a, b);
                }
                else
                {
                    result = rgt_l1ml2n(a, b);
                }
            }
            else if (b->type & nSecp256r1_MONTGOMERY)
            {
                result = rgt_l1nl2m(a, b);
            }
            else
            {
                result = rgt_l1nl2n(a, b);
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            result = rgt_l1ms2(a, b);
        }
        else
        {
            result = rgt_l1ns2(a, b);
        }
    }
    else if (b->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_MONTGOMERY)
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

void nSecp256r1_gt(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->shortVal = nSecp256r1_rgt(r, a, b);
    r->type = nSecp256r1_SHORT;
}

void nSecp256r1_leq(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
   int32_t result = nSecp256r1_rgt(r, a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = nSecp256r1_SHORT;
}

// Logical and between two elements
void nSecp256r1_land(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    int32_t is_true_a;

    if (a->type & nSecp256r1_LONG)
    {
        is_true_a = !nSecp256r1_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & nSecp256r1_LONG)
    {
        is_true_b = !nSecp256r1_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a & is_true_b;
    r->type = nSecp256r1_SHORT;
}

static inline void and_s1s2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        int32_t result = a->shortVal & b->shortVal;
        r->shortVal = result;
        r->type = nSecp256r1_SHORT;
        return;
    }

    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toLongNormal(&a_n, a);
    nSecp256r1_toLongNormal(&b_n, b);

    nSecp256r1_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1nl2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;
    nSecp256r1_rawAnd(r->longVal, a->longVal, b->longVal);
}

static inline void and_l1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element b_n;
    nSecp256r1_toNormal(&b_n, b);

    nSecp256r1_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

static inline void and_l1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);
    nSecp256r1_toNormal(&b_n, b);

    nSecp256r1_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ml2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1_toNormal(&a_n, a);

    nSecp256r1_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_s1l2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&a_n, a);
    }

    nSecp256r1_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_l1ms2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&b_n, b);
    }

    nSecp256r1_rawAnd(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void and_s1l2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&a_n, a);
    }

    nSecp256r1_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ns2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&b_n, b);
    }

    nSecp256r1_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

// Ands two elements of any kind
void nSecp256r1_band(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    and_l1ml2m(r, a, b);
                }
                else
                {
                    and_l1ml2n(r, a, b);
                }
            }
            else if (b->type & nSecp256r1_MONTGOMERY)
            {
                and_l1nl2m(r, a, b);
            }
            else
            {
                and_l1nl2n(r, a, b);
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            and_l1ms2(r, a, b);
        }
        else
        {
           and_l1ns2(r, a, b);
        }
    }
    else if (b->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_MONTGOMERY)
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

void nSecp256r1_rawZero(nSecp256r1RawElement pRawResult)
{
    std::memset(pRawResult, 0, sizeof(nSecp256r1RawElement));
}

static inline void rawShl(nSecp256r1RawElement r, nSecp256r1RawElement a, uint64_t b)
{
    if (b == 0)
    {
        nSecp256r1_rawCopy(r, a);
        return;
    }

    if (b >= 256)
    {
        nSecp256r1_rawZero(r);
        return;
    }

    nSecp256r1_rawShl(r, a, b);
}

static inline void rawShr(nSecp256r1RawElement r, nSecp256r1RawElement a, uint64_t b)
{
    if (b == 0)
    {
        nSecp256r1_rawCopy(r, a);
        return;
    }

    if (b >= 256)
    {
        nSecp256r1_rawZero(r);
        return;
    }

    nSecp256r1_rawShr(r,a, b);
}

static inline void nSecp256r1_setzero(PnSecp256r1Element r)
{
    r->type = 0;
    r->shortVal = 0;
}

static inline void do_shlcl(PnSecp256r1Element r, PnSecp256r1Element a, uint64_t b)
{
    nSecp256r1Element a_long;
    nSecp256r1_toLongNormal(&a_long, a);

    r->type = nSecp256r1_LONG;
    rawShl(r->longVal, a_long.longVal, b);
}

static inline void do_shlln(PnSecp256r1Element r, PnSecp256r1Element a, uint64_t b)
{
    r->type = nSecp256r1_LONG;
    rawShl(r->longVal, a->longVal, b);
}

static inline void do_shl(PnSecp256r1Element r, PnSecp256r1Element a, uint64_t b)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (a->type == nSecp256r1_LONGMONTGOMERY)
        {
            nSecp256r1Element a_long;
            nSecp256r1_toNormal(&a_long, a);

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
            nSecp256r1_setzero(r);
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
                r->type = nSecp256r1_SHORT;
                r->shortVal = a_shortVal;
            }
        }
    }
}

static inline void do_shrln(PnSecp256r1Element r, PnSecp256r1Element a, uint64_t b)
{
    r->type = nSecp256r1_LONG;
    rawShr(r->longVal, a->longVal, b);
}

static inline void do_shrl(PnSecp256r1Element r, PnSecp256r1Element a, uint64_t b)
{
    if (a->type == nSecp256r1_LONGMONTGOMERY)
    {
        nSecp256r1Element a_long;
        nSecp256r1_toNormal(&a_long, a);

        do_shrln(r, &a_long, b);
    }
    else
    {
        do_shrln(r, a, b);
    }
}

static inline void do_shr(PnSecp256r1Element r, PnSecp256r1Element a, uint64_t b)
{
    if (a->type & nSecp256r1_LONG)
    {
        do_shrl(r, a, b);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            nSecp256r1_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            nSecp256r1Element a_long;
            nSecp256r1_toLongNormal(&a_long, a);

            do_shrl(r, &a_long, b);
        }
        else if(b >= 31)
        {
            nSecp256r1_setzero(r);
        }
        else
        {
            a_shortVal >>= b;

            r->shortVal = a_shortVal;
            r->type = nSecp256r1_SHORT;
        }
    }
}

static inline void nSecp256r1_shr_big_shift(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    static nSecp256r1RawElement max_shift = {256};

    nSecp256r1RawElement shift;

    nSecp256r1_rawSubRegular(shift, nSecp256r1_q.longVal, b->longVal);

    if (nSecp256r1_rawCmp(shift, max_shift) >= 0)
    {
        nSecp256r1_setzero(r);
    }
    else
    {
        do_shl(r, a, shift[0]);
    }
}

static inline void nSecp256r1_shr_long(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    static nSecp256r1RawElement max_shift = {256};

    if (nSecp256r1_rawCmp(b->longVal, max_shift) >= 0)
    {
        nSecp256r1_shr_big_shift(r, a, b);
    }
    else
    {
        do_shr(r, a, b->longVal[0]);
    }
}

void nSecp256r1_shr(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (b->type & nSecp256r1_LONG)
    {
        if (b->type == nSecp256r1_LONGMONTGOMERY)
        {
            nSecp256r1Element b_long;
            nSecp256r1_toNormal(&b_long, b);

            nSecp256r1_shr_long(r, a, &b_long);
        }
        else
        {
            nSecp256r1_shr_long(r, a, b);
        }
    }
    else
    {
        int64_t b_shortVal = b->shortVal;

        if (b_shortVal < 0)
        {
            b_shortVal = -b_shortVal;

            if (b_shortVal >= 256)
            {
                nSecp256r1_setzero(r);
            }
            else
            {
                do_shl(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 256)
        {
            nSecp256r1_setzero(r);
        }
        else
        {
            do_shr(r, a, b_shortVal);
        }
    }
}

static inline void nSecp256r1_shl_big_shift(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    static nSecp256r1RawElement max_shift = {256};

    nSecp256r1RawElement shift;

    nSecp256r1_rawSubRegular(shift, nSecp256r1_q.longVal, b->longVal);

    if (nSecp256r1_rawCmp(shift, max_shift) >= 0)
    {
        nSecp256r1_setzero(r);
    }
    else
    {
        do_shr(r, a, shift[0]);
    }
}

static inline void nSecp256r1_shl_long(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    static nSecp256r1RawElement max_shift = {256};

    if (nSecp256r1_rawCmp(b->longVal, max_shift) >= 0)
    {
        nSecp256r1_shl_big_shift(r, a, b);
    }
    else
    {
        do_shl(r, a, b->longVal[0]);
    }
}

void nSecp256r1_shl(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (b->type & nSecp256r1_LONG)
    {
        if (b->type == nSecp256r1_LONGMONTGOMERY)
        {
            nSecp256r1Element b_long;
            nSecp256r1_toNormal(&b_long, b);

            nSecp256r1_shl_long(r, a, &b_long);
        }
        else
        {
            nSecp256r1_shl_long(r, a, b);
        }
    }
    else
    {
        int64_t b_shortVal = b->shortVal;

        if (b_shortVal < 0)
        {
            b_shortVal = -b_shortVal;

            if (b_shortVal >= 256)
            {
                nSecp256r1_setzero(r);
            }
            else
            {
                do_shr(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 256)
        {
            nSecp256r1_setzero(r);
        }
        else
        {
            do_shl(r, a, b_shortVal);
        }
    }
}

void nSecp256r1_square(PnSecp256r1Element r, PnSecp256r1Element a)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (a->type == nSecp256r1_LONGMONTGOMERY)
        {
            r->type = nSecp256r1_LONGMONTGOMERY;
            nSecp256r1_rawMSquare(r->longVal, a->longVal);
        }
        else
        {
            r->type = nSecp256r1_LONGMONTGOMERY;
            nSecp256r1_rawMSquare(r->longVal, a->longVal);
            nSecp256r1_rawMMul(r->longVal, r->longVal, nSecp256r1_R3.longVal);
        }
    }
    else
    {
        int64_t result;

        int overflow = nSecp256r1_rawSMul(&result, a->shortVal, a->shortVal);

        if (overflow)
        {
            nSecp256r1_rawCopyS2L(r->longVal, result);
            r->type = nSecp256r1_LONG;
            r->shortVal = 0;
        }
        else
        {
            // done the same way as in intel asm implementation
            r->shortVal = (int32_t)result;
            r->type = nSecp256r1_SHORT;
            //

            nSecp256r1_rawCopyS2L(r->longVal, result);
            r->type = nSecp256r1_LONG;
            r->shortVal = 0;
        }
    }
}

static inline void or_s1s2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal | b->shortVal;
        r->type = nSecp256r1_SHORT;
        return;
    }

    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toLongNormal(&a_n, a);
    nSecp256r1_toLongNormal(&b_n, b);

    nSecp256r1_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&a_n, a);
    }

    nSecp256r1_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&a_n, a);
    }

    nSecp256r1_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ns2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&b_n, b);
    }

    nSecp256r1_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ms2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&b_n, b);
    }

    nSecp256r1_rawOr(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void or_l1nl2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;
    nSecp256r1_rawOr(r->longVal, a->longVal, b->longVal);
}

static inline void or_l1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element b_n;
    nSecp256r1_toNormal(&b_n, b);

    nSecp256r1_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ml2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1_toNormal(&a_n, a);

    nSecp256r1_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);
    nSecp256r1_toNormal(&b_n, b);

    nSecp256r1_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}


void nSecp256r1_bor(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    or_l1ml2m(r, a, b);
                }
                else
                {
                    or_l1ml2n(r, a, b);
                }
            }
            else if (b->type & nSecp256r1_MONTGOMERY)
            {
                or_l1nl2m(r, a, b);
            }
            else
            {
                or_l1nl2n(r, a, b);
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            or_l1ms2(r, a, b);
        }
        else
        {
           or_l1ns2(r, a, b);
        }
    }
    else if (b->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_MONTGOMERY)
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

static inline void xor_s1s2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal ^ b->shortVal;
        r->type = nSecp256r1_SHORT;
        return;
    }

    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toLongNormal(&a_n, a);
    nSecp256r1_toLongNormal(&b_n, b);

    nSecp256r1_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_s1l2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&a_n, a);
    }

    nSecp256r1_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_s1l2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&a_n, a);
    }

    nSecp256r1_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_l1ns2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&b_n, b);
    }

    nSecp256r1_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ms2(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        nSecp256r1_toLongNormal(&b_n, b);
    }

    nSecp256r1_rawXor(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void xor_l1nl2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;
    nSecp256r1_rawXor(r->longVal, a->longVal, b->longVal);
}

static inline void xor_l1nl2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element b_n;
    nSecp256r1_toNormal(&b_n, b);

    nSecp256r1_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ml2n(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1_toNormal(&a_n, a);

    nSecp256r1_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_l1ml2m(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    r->type = nSecp256r1_LONG;

    nSecp256r1Element a_n;
    nSecp256r1Element b_n;

    nSecp256r1_toNormal(&a_n, a);
    nSecp256r1_toNormal(&b_n, b);

    nSecp256r1_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

void nSecp256r1_bxor(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b)
{
    if (a->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_LONG)
        {
            if (a->type & nSecp256r1_MONTGOMERY)
            {
                if (b->type & nSecp256r1_MONTGOMERY)
                {
                    xor_l1ml2m(r, a, b);
                }
                else
                {
                    xor_l1ml2n(r, a, b);
                }
            }
            else if (b->type & nSecp256r1_MONTGOMERY)
            {
                xor_l1nl2m(r, a, b);
            }
            else
            {
                xor_l1nl2n(r, a, b);
            }
        }
        else if (a->type & nSecp256r1_MONTGOMERY)
        {
            xor_l1ms2(r, a, b);
        }
        else
        {
           xor_l1ns2(r, a, b);
        }
    }
    else if (b->type & nSecp256r1_LONG)
    {
        if (b->type & nSecp256r1_MONTGOMERY)
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

void nSecp256r1_bnot(PnSecp256r1Element r, PnSecp256r1Element a)
{
    r->type = nSecp256r1_LONG;

    if (a->type == nSecp256r1_LONG)
    {
        if (a->type & nSecp256r1_MONTGOMERY)
        {
            nSecp256r1Element a_n;
            nSecp256r1_toNormal(&a_n, a);

            nSecp256r1_rawNot(r->longVal, a_n.longVal);
        }
        else
        {
            nSecp256r1_rawNot(r->longVal, a->longVal);
        }
    }
    else
    {
        nSecp256r1Element a_n;
        nSecp256r1_toLongNormal(&a_n, a);

        nSecp256r1_rawNot(r->longVal, a_n.longVal);
    }
}
