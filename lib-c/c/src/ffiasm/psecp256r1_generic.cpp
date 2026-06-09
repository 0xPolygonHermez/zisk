#include "psecp256r1.hpp"
#include <cstdint>
#include <cstring>
#include <cassert>

pSecp256r1Element pSecp256r1_q  = {0, 0x80000000, {0xffffffffffffffff,0x00000000ffffffff,0x0000000000000000,0xffffffff00000001}};
pSecp256r1Element pSecp256r1_R2 = {0, 0x80000000, {0x0000000000000003,0xfffffffbffffffff,0xfffffffffffffffe,0x00000004fffffffd}};
pSecp256r1Element pSecp256r1_R3 = {0, 0x80000000, {0xfffffffd0000000a,0xffffffedfffffff7,0x00000005fffffffc,0x0000001800000001}};

static pSecp256r1RawElement half = {0xffffffffffffffff,0x000000007fffffff,0x8000000000000000,0x7fffffff80000000};
static pSecp256r1RawElement zero = {0};


void pSecp256r1_copy(PpSecp256r1Element r, const PpSecp256r1Element a)
{
    *r = *a;
}

void pSecp256r1_toNormal(PpSecp256r1Element r, PpSecp256r1Element a)
{
    if (a->type == pSecp256r1_LONGMONTGOMERY)
    {
        r->type = pSecp256r1_LONG;
        pSecp256r1_rawFromMontgomery(r->longVal, a->longVal);
    }
    else
    {
        pSecp256r1_copy(r, a);
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

static inline int pSecp256r1_rawSMul(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a * b;

    return has_mul32_overflow(*r);
}

static inline void mul_s1s2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    int64_t result;

    int overflow = pSecp256r1_rawSMul(&result, a->shortVal, b->shortVal);

    if (overflow)
    {
        pSecp256r1_rawCopyS2L(r->longVal, result);
        r->type = pSecp256r1_LONG;
        r->shortVal = 0;
    }
    else
    {
        // done the same way as in intel asm implementation
        r->shortVal = (int32_t)result;
        r->type = pSecp256r1_SHORT;
        //

        pSecp256r1_rawCopyS2L(r->longVal, result);
        r->type = pSecp256r1_LONG;
        r->shortVal = 0;
    }
}

static inline void mul_l1nl2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
    pSecp256r1_rawMMul(r->longVal, r->longVal, pSecp256r1_R3.longVal);
}

static inline void mul_l1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;
    pSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;
    pSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;
    pSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ns2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        pSecp256r1_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        pSecp256r1_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        pSecp256r1_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }

    pSecp256r1_rawMMul(r->longVal, r->longVal, pSecp256r1_R3.longVal);
}

static inline void mul_s1nl2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        pSecp256r1_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        pSecp256r1_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        pSecp256r1_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }

    pSecp256r1_rawMMul(r->longVal, r->longVal, pSecp256r1_R3.longVal);
}

static inline void mul_l1ms2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        pSecp256r1_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        pSecp256r1_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        pSecp256r1_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void mul_s1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        pSecp256r1_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        pSecp256r1_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        pSecp256r1_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }
}

static inline void mul_l1ns2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;
    pSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ms2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;
    pSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;
    pSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;
    pSecp256r1_rawMMul(r->longVal, a->longVal, b->longVal);
}

void pSecp256r1_mul(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
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
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    mul_l1nl2m(r, a, b);
                }
                else
                {
                    mul_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            if (b->type & pSecp256r1_MONTGOMERY)
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
            if (b->type & pSecp256r1_MONTGOMERY)
            {
                mul_l1ns2m(r, a, b);
            }
            else
            {
                mul_l1ns2n(r, a, b);
            }
        }
    }
    else if (b->type & pSecp256r1_LONG)
    {
        if (a->type & pSecp256r1_MONTGOMERY)
        {
            if (b->type & pSecp256r1_MONTGOMERY)
            {
                mul_s1ml2m(r, a, b);
            }
            else
            {
                mul_s1ml2n(r,a, b);
            }
        }
        else if (b->type & pSecp256r1_MONTGOMERY)
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

void pSecp256r1_toLongNormal(PpSecp256r1Element r, PpSecp256r1Element a)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (a->type & pSecp256r1_MONTGOMERY)
        {
            pSecp256r1_rawFromMontgomery(r->longVal, a->longVal);
            r->type = pSecp256r1_LONG;
        }
        else
        {
            pSecp256r1_copy(r, a);
        }
    }
    else
    {
        pSecp256r1_rawCopyS2L(r->longVal, a->shortVal);
        r->type = pSecp256r1_LONG;
        r->shortVal = 0;
    }
}

void pSecp256r1_toMontgomery(PpSecp256r1Element r, PpSecp256r1Element a)
{
    if (a->type & pSecp256r1_MONTGOMERY)
    {
        pSecp256r1_copy(r, a);
    }
    else if (a->type & pSecp256r1_LONG)
    {
        r->shortVal = a->shortVal;

        pSecp256r1_rawMMul(r->longVal, a->longVal, pSecp256r1_R2.longVal);

        r->type = pSecp256r1_LONGMONTGOMERY;
    }
    else if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
       pSecp256r1_rawMMul1(r->longVal, pSecp256r1_R2.longVal, -a_shortVal);
       pSecp256r1_rawNeg(r->longVal, r->longVal);

       r->type = pSecp256r1_SHORTMONTGOMERY;
    }
    else
    {
        pSecp256r1_rawMMul1(r->longVal, pSecp256r1_R2.longVal, a->shortVal);

        r->type = pSecp256r1_SHORTMONTGOMERY;
    }
}

void pSecp256r1_copyn(PpSecp256r1Element r, PpSecp256r1Element a, int n)
{
    std::memcpy(r, a, n * sizeof(pSecp256r1Element));
}

static inline int has_add32_overflow(int64_t val)
{
    int64_t signs = (val >> 31) & 0x3;

    return signs == 1 || signs == 2;
}

static inline int pSecp256r1_rawSSub(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a - b;

    return has_add32_overflow(*r);
}

static inline void sub_s1s2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    int64_t diff;

    int overflow = pSecp256r1_rawSSub(&diff, a->shortVal, b->shortVal);

    if (overflow)
    {
        pSecp256r1_rawCopyS2L(r->longVal, diff);
        r->type = pSecp256r1_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = pSecp256r1_SHORT;
        r->shortVal = (int32_t)diff;
    }
}

static inline void sub_l1nl2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1Element a_m;
    pSecp256r1_toMontgomery(&a_m, a);

    pSecp256r1_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1ml2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1Element b_m;
    pSecp256r1_toMontgomery(&b_m, b);

    pSecp256r1_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1l2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    if (a->shortVal >= 0)
    {
        pSecp256r1_rawSubSL(r->longVal, a->shortVal, b->longVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        pSecp256r1_rawNegLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void sub_l1ms2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1Element b_m;
    pSecp256r1_toMontgomery(&b_m, b);

    pSecp256r1_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1Element a_m;
    pSecp256r1_toMontgomery(&a_m, a);

    pSecp256r1_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ns2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        pSecp256r1_rawAddLS(r->longVal, a->longVal, -b_shortVal);
    }
    else
    {
        pSecp256r1_rawSubLS(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void sub_l1ms2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_s1ml2m(PpSecp256r1Element r,PpSecp256r1Element a,PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1_rawSub(r->longVal, a->longVal, b->longVal);
}

void pSecp256r1_sub(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    sub_l1ml2m(r, a, b);
                }
                else
                {
                    sub_l1ml2n(r, a, b);
                }
            }
            else if (b->type & pSecp256r1_MONTGOMERY)
            {
                sub_l1nl2m(r, a, b);
            }
            else
            {
                sub_l1nl2n(r, a, b);
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            if (b->type & pSecp256r1_MONTGOMERY)
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
    else if (b->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_MONTGOMERY)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
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

static inline int pSecp256r1_rawSAdd(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a + b;

    return has_add32_overflow(*r);
}

static inline void add_s1s2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    int64_t sum;

    int overflow = pSecp256r1_rawSAdd(&sum, a->shortVal, b->shortVal);

    if (overflow)
    {
        pSecp256r1_rawCopyS2L(r->longVal, sum);
        r->type = pSecp256r1_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = pSecp256r1_SHORT;
        r->shortVal = (int32_t)sum;
    }
}

static inline void add_l1nl2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1Element a_m;
    pSecp256r1_toMontgomery(&a_m, a);

    pSecp256r1_rawAdd(r->longVal, a_m.longVal, b->longVal);
}

static inline void add_l1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;
    pSecp256r1_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1ml2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1Element b_m;
    pSecp256r1_toMontgomery(&b_m, b);

    pSecp256r1_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1l2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    if (a->shortVal >= 0)
    {
        pSecp256r1_rawAddLS(r->longVal, b->longVal, a->shortVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        pSecp256r1_rawSubLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void add_l1ms2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element b_m;

    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1_toMontgomery(&b_m, b);

    pSecp256r1_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1Element m_a;
    pSecp256r1_toMontgomery(&m_a, a);

    pSecp256r1_rawAdd(r->longVal, m_a.longVal, b->longVal);
}

static inline void add_l1ns2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    if (b->shortVal >= 0)
    {
        pSecp256r1_rawAddLS(r->longVal, a->longVal, b->shortVal);
    }
    else
    {
        int64_t b_shortVal = b->shortVal;
        pSecp256r1_rawSubLS(r->longVal, a->longVal, -b_shortVal);
    }
}

static inline void add_l1ms2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_s1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONGMONTGOMERY;

    pSecp256r1_rawAdd(r->longVal, a->longVal, b->longVal);
}

void pSecp256r1_add(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
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
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    add_l1nl2m(r, a, b);
                }
                else
                {
                    add_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            if (b->type & pSecp256r1_MONTGOMERY)
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
    else if (b->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_MONTGOMERY)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
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

int pSecp256r1_isTrue(PpSecp256r1Element pE)
{
    int result;

    if (pE->type & pSecp256r1_LONG)
    {
        result = !pSecp256r1_rawIsZero(pE->longVal);
    }
    else
    {
        result = pE->shortVal != 0;
    }

    return result;
}

int pSecp256r1_longNeg(PpSecp256r1Element pE)
{
    if(pSecp256r1_rawCmp(pE->longVal, pSecp256r1_q.longVal) >= 0)
    {
       pSecp256r1_longErr();
       return 0;
    }

    int64_t result = pE->longVal[0] - pSecp256r1_q.longVal[0];

    int64_t is_long = (result >> 31) + 1;

    if(is_long)
    {
       pSecp256r1_longErr();
       return 0;
    }

    return result;
}

int pSecp256r1_longNormal(PpSecp256r1Element pE)
{
    uint64_t is_long = 0;
    uint64_t result;

    result = pE->longVal[0];

    is_long = result >> 31;

    if (is_long)
    {
         return pSecp256r1_longNeg(pE);
    }

    if (memcmp(&pE->longVal[1], zero, (sizeof(pE->longVal) - sizeof(pE->longVal[0]))))
    {
        return pSecp256r1_longNeg(pE);
    }

    return result;
}

// Convert a 64 bit integer to a long format field element
int pSecp256r1_toInt(PpSecp256r1Element pE)
{
    int result;

    if (pE->type & pSecp256r1_LONG)
    {
       if (pE->type & pSecp256r1_MONTGOMERY)
       {
           pSecp256r1Element e_n;
           pSecp256r1_toNormal(&e_n, pE);

           result = pSecp256r1_longNormal(&e_n);
       }
       else
       {
           result = pSecp256r1_longNormal(pE);
       }
    }
    else
    {
        result = pE->shortVal;
    }

    return result;
}

static inline int rlt_s1s2(PpSecp256r1Element a, PpSecp256r1Element b)
{
    return (a->shortVal < b->shortVal) ? 1 : 0;
}

static inline int rltRawL1L2(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    int result = pSecp256r1_rawCmp(pRawB, pRawA);

    return result > 0 ? 1 : 0;
}

static inline int rltl1l2_n1(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    int result = pSecp256r1_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rltRawL1L2(pRawA, pRawB);
    }

     return 1;
}

static inline int rltl1l2_p1(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    int result = pSecp256r1_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 0;
    }

    return rltRawL1L2(pRawA, pRawB);
}

static inline int rltL1L2(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    int result = pSecp256r1_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rltl1l2_n1(pRawA, pRawB);
    }

    return rltl1l2_p1(pRawA, pRawB);
}

static inline int rlt_l1nl2n(PpSecp256r1Element a, PpSecp256r1Element b)
{
    return rltL1L2(a->longVal, b->longVal);
}

static inline int rlt_l1nl2m(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

static inline int rlt_l1ml2m(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);
    pSecp256r1_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ml2n(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;

    pSecp256r1_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_s1l2n(PpSecp256r1Element a,PpSecp256r1Element b)
{
    pSecp256r1Element a_n;

    pSecp256r1_toLongNormal(&a_n,a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_l1ms2(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1Element b_ln;

    pSecp256r1_toLongNormal(&b_ln ,b);
    pSecp256r1_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b_ln.longVal);
}

static inline int rlt_s1l2m(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toLongNormal(&a_n, a);
    pSecp256r1_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ns2(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element b_n;

    pSecp256r1_toLongNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

int32_t pSecp256r1_rlt(PpSecp256r1Element a, PpSecp256r1Element b)
{
    int32_t result;

    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    result = rlt_l1ml2m(a, b);
                }
                else
                {
                    result = rlt_l1ml2n(a, b);
                }
            }
            else if (b->type & pSecp256r1_MONTGOMERY)
            {
                result = rlt_l1nl2m(a, b);
            }
            else
            {
                result = rlt_l1nl2n(a, b);
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            result = rlt_l1ms2(a, b);
        }
        else
        {
            result = rlt_l1ns2(a, b);
        }
    }
    else if (b->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_MONTGOMERY)
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

void pSecp256r1_lt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->shortVal = pSecp256r1_rlt(a, b);
    r->type = pSecp256r1_SHORT;
}

void pSecp256r1_geq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
   int32_t result = pSecp256r1_rlt(a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = pSecp256r1_SHORT;
}

static inline int pSecp256r1_rawSNeg(int64_t *r, int32_t a)
{
    *r = -(int64_t)a;

    return has_add32_overflow(*r);
}

void pSecp256r1_neg(PpSecp256r1Element r, PpSecp256r1Element a)
{
    if (a->type & pSecp256r1_LONG)
    {
        r->type = a->type;
        r->shortVal = a->shortVal;
        pSecp256r1_rawNeg(r->longVal, a->longVal);
    }
    else
    {
        int64_t a_shortVal;

        int overflow = pSecp256r1_rawSNeg(&a_shortVal, a->shortVal);

        if (overflow)
        {
            pSecp256r1_rawCopyS2L(r->longVal, a_shortVal);
            r->type = pSecp256r1_LONG;
            r->shortVal = 0;
        }
        else
        {
            r->type = pSecp256r1_SHORT;
            r->shortVal = (int32_t)a_shortVal;
        }
    }
}

static inline int reqL1L2(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    return pSecp256r1_rawCmp(pRawB, pRawA) == 0;
}

static inline int req_s1s2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    return (a->shortVal == b->shortVal) ? 1 : 0;
}

static inline int req_l1nl2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_m;
    pSecp256r1_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1ml2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element b_m;
    pSecp256r1_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1_toLongNormal(&a_n, a);

    return reqL1L2(a_n.longVal, b->longVal);
}

static inline int req_l1ms2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element b_m;
    pSecp256r1_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_m;
    pSecp256r1_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ns2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element b_n;
    pSecp256r1_toLongNormal(&b_n, b);

    return reqL1L2(a->longVal, b_n.longVal);
}

// Compares two elements of any kind
int pSecp256r1_req(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    int result;

    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    result = req_l1ml2m(r, a, b);
                }
                else
                {
                    result = req_l1ml2n(r, a, b);
                }
            }
            else if (b->type & pSecp256r1_MONTGOMERY)
            {
                result = req_l1nl2m(r, a, b);
            }
            else
            {
                result = req_l1nl2n(r, a, b);
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            result = req_l1ms2(r, a, b);
        }
        else
        {
            result = req_l1ns2(r, a, b);
        }
    }
    else if (b->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_MONTGOMERY)
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

void pSecp256r1_eq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->shortVal = pSecp256r1_req(r, a, b);
    r->type = pSecp256r1_SHORT;
}

void pSecp256r1_neq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    int result = pSecp256r1_req(r, a, b);

    r->shortVal = result ^ 0x1;
    r->type = pSecp256r1_SHORT;
}

// Logical or between two elements
void pSecp256r1_lor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    int32_t is_true_a;

    if (a->type & pSecp256r1_LONG)
    {
        is_true_a = !pSecp256r1_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & pSecp256r1_LONG)
    {
        is_true_b = !pSecp256r1_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a | is_true_b;
    r->type = pSecp256r1_SHORT;
}

void pSecp256r1_lnot(PpSecp256r1Element r, PpSecp256r1Element a)
{
    if (a->type & pSecp256r1_LONG)
    {
        r->shortVal = pSecp256r1_rawIsZero(a->longVal);
    }
    else
    {
        r->shortVal = a->shortVal ? 0 : 1;
    }

    r->type = pSecp256r1_SHORT;
}


static inline int rgt_s1s2(PpSecp256r1Element a, PpSecp256r1Element b)
{
    return (a->shortVal > b->shortVal) ? 1 : 0;
}

static inline int rgtRawL1L2(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    int result = pSecp256r1_rawCmp(pRawB, pRawA);

    return (result < 0) ? 1 : 0;
}

static inline int rgtl1l2_n1(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    int result = pSecp256r1_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rgtRawL1L2(pRawA, pRawB);
    }
    return 0;
}

static inline int rgtl1l2_p1(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    int result = pSecp256r1_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 1;
    }
    return rgtRawL1L2(pRawA, pRawB);
}

static inline int rgtL1L2(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB)
{
    int result = pSecp256r1_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rgtl1l2_n1(pRawA, pRawB);
    }

    return rgtl1l2_p1(pRawA, pRawB);
}

static inline int rgt_l1nl2n(PpSecp256r1Element a, PpSecp256r1Element b)
{
    return rgtL1L2(a->longVal, b->longVal);
}

static inline int rgt_l1nl2m(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element b_n;
    pSecp256r1_toNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

static inline int rgt_l1ml2m(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);
    pSecp256r1_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ml2n(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1_toNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_s1l2n(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1_toLongNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_l1ms2(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);
    pSecp256r1_toLongNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_s1l2m(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toLongNormal(&a_n, a);
    pSecp256r1_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ns2(PpSecp256r1Element a, PpSecp256r1Element b)
{
    pSecp256r1Element b_n;
    pSecp256r1_toLongNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

int pSecp256r1_rgt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    int result = 0;

    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    result = rgt_l1ml2m(a, b);
                }
                else
                {
                    result = rgt_l1ml2n(a, b);
                }
            }
            else if (b->type & pSecp256r1_MONTGOMERY)
            {
                result = rgt_l1nl2m(a, b);
            }
            else
            {
                result = rgt_l1nl2n(a, b);
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            result = rgt_l1ms2(a, b);
        }
        else
        {
            result = rgt_l1ns2(a, b);
        }
    }
    else if (b->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_MONTGOMERY)
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

void pSecp256r1_gt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->shortVal = pSecp256r1_rgt(r, a, b);
    r->type = pSecp256r1_SHORT;
}

void pSecp256r1_leq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
   int32_t result = pSecp256r1_rgt(r, a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = pSecp256r1_SHORT;
}

// Logical and between two elements
void pSecp256r1_land(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    int32_t is_true_a;

    if (a->type & pSecp256r1_LONG)
    {
        is_true_a = !pSecp256r1_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & pSecp256r1_LONG)
    {
        is_true_b = !pSecp256r1_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a & is_true_b;
    r->type = pSecp256r1_SHORT;
}

static inline void and_s1s2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        int32_t result = a->shortVal & b->shortVal;
        r->shortVal = result;
        r->type = pSecp256r1_SHORT;
        return;
    }

    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toLongNormal(&a_n, a);
    pSecp256r1_toLongNormal(&b_n, b);

    pSecp256r1_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1nl2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;
    pSecp256r1_rawAnd(r->longVal, a->longVal, b->longVal);
}

static inline void and_l1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element b_n;
    pSecp256r1_toNormal(&b_n, b);

    pSecp256r1_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

static inline void and_l1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);
    pSecp256r1_toNormal(&b_n, b);

    pSecp256r1_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ml2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1_toNormal(&a_n, a);

    pSecp256r1_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_s1l2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&a_n, a);
    }

    pSecp256r1_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_l1ms2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&b_n, b);
    }

    pSecp256r1_rawAnd(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void and_s1l2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&a_n, a);
    }

    pSecp256r1_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ns2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&b_n, b);
    }

    pSecp256r1_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

// Ands two elements of any kind
void pSecp256r1_band(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    and_l1ml2m(r, a, b);
                }
                else
                {
                    and_l1ml2n(r, a, b);
                }
            }
            else if (b->type & pSecp256r1_MONTGOMERY)
            {
                and_l1nl2m(r, a, b);
            }
            else
            {
                and_l1nl2n(r, a, b);
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            and_l1ms2(r, a, b);
        }
        else
        {
           and_l1ns2(r, a, b);
        }
    }
    else if (b->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_MONTGOMERY)
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

void pSecp256r1_rawZero(pSecp256r1RawElement pRawResult)
{
    std::memset(pRawResult, 0, sizeof(pSecp256r1RawElement));
}

static inline void rawShl(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b)
{
    if (b == 0)
    {
        pSecp256r1_rawCopy(r, a);
        return;
    }

    if (b >= 256)
    {
        pSecp256r1_rawZero(r);
        return;
    }

    pSecp256r1_rawShl(r, a, b);
}

static inline void rawShr(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b)
{
    if (b == 0)
    {
        pSecp256r1_rawCopy(r, a);
        return;
    }

    if (b >= 256)
    {
        pSecp256r1_rawZero(r);
        return;
    }

    pSecp256r1_rawShr(r,a, b);
}

static inline void pSecp256r1_setzero(PpSecp256r1Element r)
{
    r->type = 0;
    r->shortVal = 0;
}

static inline void do_shlcl(PpSecp256r1Element r, PpSecp256r1Element a, uint64_t b)
{
    pSecp256r1Element a_long;
    pSecp256r1_toLongNormal(&a_long, a);

    r->type = pSecp256r1_LONG;
    rawShl(r->longVal, a_long.longVal, b);
}

static inline void do_shlln(PpSecp256r1Element r, PpSecp256r1Element a, uint64_t b)
{
    r->type = pSecp256r1_LONG;
    rawShl(r->longVal, a->longVal, b);
}

static inline void do_shl(PpSecp256r1Element r, PpSecp256r1Element a, uint64_t b)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (a->type == pSecp256r1_LONGMONTGOMERY)
        {
            pSecp256r1Element a_long;
            pSecp256r1_toNormal(&a_long, a);

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
            pSecp256r1_setzero(r);
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
                r->type = pSecp256r1_SHORT;
                r->shortVal = a_shortVal;
            }
        }
    }
}

static inline void do_shrln(PpSecp256r1Element r, PpSecp256r1Element a, uint64_t b)
{
    r->type = pSecp256r1_LONG;
    rawShr(r->longVal, a->longVal, b);
}

static inline void do_shrl(PpSecp256r1Element r, PpSecp256r1Element a, uint64_t b)
{
    if (a->type == pSecp256r1_LONGMONTGOMERY)
    {
        pSecp256r1Element a_long;
        pSecp256r1_toNormal(&a_long, a);

        do_shrln(r, &a_long, b);
    }
    else
    {
        do_shrln(r, a, b);
    }
}

static inline void do_shr(PpSecp256r1Element r, PpSecp256r1Element a, uint64_t b)
{
    if (a->type & pSecp256r1_LONG)
    {
        do_shrl(r, a, b);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            pSecp256r1_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            pSecp256r1Element a_long;
            pSecp256r1_toLongNormal(&a_long, a);

            do_shrl(r, &a_long, b);
        }
        else if(b >= 31)
        {
            pSecp256r1_setzero(r);
        }
        else
        {
            a_shortVal >>= b;

            r->shortVal = a_shortVal;
            r->type = pSecp256r1_SHORT;
        }
    }
}

static inline void pSecp256r1_shr_big_shift(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    static pSecp256r1RawElement max_shift = {256};

    pSecp256r1RawElement shift;

    pSecp256r1_rawSubRegular(shift, pSecp256r1_q.longVal, b->longVal);

    if (pSecp256r1_rawCmp(shift, max_shift) >= 0)
    {
        pSecp256r1_setzero(r);
    }
    else
    {
        do_shl(r, a, shift[0]);
    }
}

static inline void pSecp256r1_shr_long(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    static pSecp256r1RawElement max_shift = {256};

    if (pSecp256r1_rawCmp(b->longVal, max_shift) >= 0)
    {
        pSecp256r1_shr_big_shift(r, a, b);
    }
    else
    {
        do_shr(r, a, b->longVal[0]);
    }
}

void pSecp256r1_shr(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (b->type & pSecp256r1_LONG)
    {
        if (b->type == pSecp256r1_LONGMONTGOMERY)
        {
            pSecp256r1Element b_long;
            pSecp256r1_toNormal(&b_long, b);

            pSecp256r1_shr_long(r, a, &b_long);
        }
        else
        {
            pSecp256r1_shr_long(r, a, b);
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
                pSecp256r1_setzero(r);
            }
            else
            {
                do_shl(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 256)
        {
            pSecp256r1_setzero(r);
        }
        else
        {
            do_shr(r, a, b_shortVal);
        }
    }
}

static inline void pSecp256r1_shl_big_shift(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    static pSecp256r1RawElement max_shift = {256};

    pSecp256r1RawElement shift;

    pSecp256r1_rawSubRegular(shift, pSecp256r1_q.longVal, b->longVal);

    if (pSecp256r1_rawCmp(shift, max_shift) >= 0)
    {
        pSecp256r1_setzero(r);
    }
    else
    {
        do_shr(r, a, shift[0]);
    }
}

static inline void pSecp256r1_shl_long(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    static pSecp256r1RawElement max_shift = {256};

    if (pSecp256r1_rawCmp(b->longVal, max_shift) >= 0)
    {
        pSecp256r1_shl_big_shift(r, a, b);
    }
    else
    {
        do_shl(r, a, b->longVal[0]);
    }
}

void pSecp256r1_shl(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (b->type & pSecp256r1_LONG)
    {
        if (b->type == pSecp256r1_LONGMONTGOMERY)
        {
            pSecp256r1Element b_long;
            pSecp256r1_toNormal(&b_long, b);

            pSecp256r1_shl_long(r, a, &b_long);
        }
        else
        {
            pSecp256r1_shl_long(r, a, b);
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
                pSecp256r1_setzero(r);
            }
            else
            {
                do_shr(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 256)
        {
            pSecp256r1_setzero(r);
        }
        else
        {
            do_shl(r, a, b_shortVal);
        }
    }
}

void pSecp256r1_square(PpSecp256r1Element r, PpSecp256r1Element a)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (a->type == pSecp256r1_LONGMONTGOMERY)
        {
            r->type = pSecp256r1_LONGMONTGOMERY;
            pSecp256r1_rawMSquare(r->longVal, a->longVal);
        }
        else
        {
            r->type = pSecp256r1_LONGMONTGOMERY;
            pSecp256r1_rawMSquare(r->longVal, a->longVal);
            pSecp256r1_rawMMul(r->longVal, r->longVal, pSecp256r1_R3.longVal);
        }
    }
    else
    {
        int64_t result;

        int overflow = pSecp256r1_rawSMul(&result, a->shortVal, a->shortVal);

        if (overflow)
        {
            pSecp256r1_rawCopyS2L(r->longVal, result);
            r->type = pSecp256r1_LONG;
            r->shortVal = 0;
        }
        else
        {
            // done the same way as in intel asm implementation
            r->shortVal = (int32_t)result;
            r->type = pSecp256r1_SHORT;
            //

            pSecp256r1_rawCopyS2L(r->longVal, result);
            r->type = pSecp256r1_LONG;
            r->shortVal = 0;
        }
    }
}

static inline void or_s1s2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal | b->shortVal;
        r->type = pSecp256r1_SHORT;
        return;
    }

    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toLongNormal(&a_n, a);
    pSecp256r1_toLongNormal(&b_n, b);

    pSecp256r1_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&a_n, a);
    }

    pSecp256r1_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&a_n, a);
    }

    pSecp256r1_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ns2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&b_n, b);
    }

    pSecp256r1_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ms2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&b_n, b);
    }

    pSecp256r1_rawOr(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void or_l1nl2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;
    pSecp256r1_rawOr(r->longVal, a->longVal, b->longVal);
}

static inline void or_l1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element b_n;
    pSecp256r1_toNormal(&b_n, b);

    pSecp256r1_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ml2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1_toNormal(&a_n, a);

    pSecp256r1_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);
    pSecp256r1_toNormal(&b_n, b);

    pSecp256r1_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}


void pSecp256r1_bor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    or_l1ml2m(r, a, b);
                }
                else
                {
                    or_l1ml2n(r, a, b);
                }
            }
            else if (b->type & pSecp256r1_MONTGOMERY)
            {
                or_l1nl2m(r, a, b);
            }
            else
            {
                or_l1nl2n(r, a, b);
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            or_l1ms2(r, a, b);
        }
        else
        {
           or_l1ns2(r, a, b);
        }
    }
    else if (b->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_MONTGOMERY)
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

static inline void xor_s1s2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal ^ b->shortVal;
        r->type = pSecp256r1_SHORT;
        return;
    }

    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toLongNormal(&a_n, a);
    pSecp256r1_toLongNormal(&b_n, b);

    pSecp256r1_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_s1l2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&a_n, a);
    }

    pSecp256r1_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_s1l2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&a_n, a);
    }

    pSecp256r1_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_l1ns2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&b_n, b);
    }

    pSecp256r1_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ms2(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        pSecp256r1_toLongNormal(&b_n, b);
    }

    pSecp256r1_rawXor(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void xor_l1nl2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;
    pSecp256r1_rawXor(r->longVal, a->longVal, b->longVal);
}

static inline void xor_l1nl2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element b_n;
    pSecp256r1_toNormal(&b_n, b);

    pSecp256r1_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ml2n(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1_toNormal(&a_n, a);

    pSecp256r1_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_l1ml2m(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    r->type = pSecp256r1_LONG;

    pSecp256r1Element a_n;
    pSecp256r1Element b_n;

    pSecp256r1_toNormal(&a_n, a);
    pSecp256r1_toNormal(&b_n, b);

    pSecp256r1_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

void pSecp256r1_bxor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b)
{
    if (a->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_LONG)
        {
            if (a->type & pSecp256r1_MONTGOMERY)
            {
                if (b->type & pSecp256r1_MONTGOMERY)
                {
                    xor_l1ml2m(r, a, b);
                }
                else
                {
                    xor_l1ml2n(r, a, b);
                }
            }
            else if (b->type & pSecp256r1_MONTGOMERY)
            {
                xor_l1nl2m(r, a, b);
            }
            else
            {
                xor_l1nl2n(r, a, b);
            }
        }
        else if (a->type & pSecp256r1_MONTGOMERY)
        {
            xor_l1ms2(r, a, b);
        }
        else
        {
           xor_l1ns2(r, a, b);
        }
    }
    else if (b->type & pSecp256r1_LONG)
    {
        if (b->type & pSecp256r1_MONTGOMERY)
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

void pSecp256r1_bnot(PpSecp256r1Element r, PpSecp256r1Element a)
{
    r->type = pSecp256r1_LONG;

    if (a->type == pSecp256r1_LONG)
    {
        if (a->type & pSecp256r1_MONTGOMERY)
        {
            pSecp256r1Element a_n;
            pSecp256r1_toNormal(&a_n, a);

            pSecp256r1_rawNot(r->longVal, a_n.longVal);
        }
        else
        {
            pSecp256r1_rawNot(r->longVal, a->longVal);
        }
    }
    else
    {
        pSecp256r1Element a_n;
        pSecp256r1_toLongNormal(&a_n, a);

        pSecp256r1_rawNot(r->longVal, a_n.longVal);
    }
}
