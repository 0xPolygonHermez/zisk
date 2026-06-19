#include "fec.hpp"
#include <cstdint>
#include <cstring>
#include <cassert>

FecElement Fec_q  = {0, 0x80000000, {0xfffffffefffffc2f,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
FecElement Fec_R2 = {0, 0x80000000, {0x000007a2000e90a1,0x0000000000000001,0x0000000000000000,0x0000000000000000}};
FecElement Fec_R3 = {0, 0x80000000, {0x002bb1e33795f671,0x0000000100000b73,0x0000000000000000,0x0000000000000000}};

static FecRawElement half = {0xffffffff7ffffe17,0xffffffffffffffff,0xffffffffffffffff,0x7fffffffffffffff};
static FecRawElement zero = {0};


void Fec_copy(PFecElement r, const PFecElement a)
{
    *r = *a;
}

void Fec_toNormal(PFecElement r, PFecElement a)
{
    if (a->type == Fec_LONGMONTGOMERY)
    {
        r->type = Fec_LONG;
        Fec_rawFromMontgomery(r->longVal, a->longVal);
    }
    else
    {
        Fec_copy(r, a);
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

static inline int Fec_rawSMul(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a * b;

    return has_mul32_overflow(*r);
}

static inline void mul_s1s2(PFecElement r, PFecElement a, PFecElement b)
{
    int64_t result;

    int overflow = Fec_rawSMul(&result, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fec_rawCopyS2L(r->longVal, result);
        r->type = Fec_LONG;
        r->shortVal = 0;
    }
    else
    {
        // done the same way as in intel asm implementation
        r->shortVal = (int32_t)result;
        r->type = Fec_SHORT;
        //

        Fec_rawCopyS2L(r->longVal, result);
        r->type = Fec_LONG;
        r->shortVal = 0;
    }
}

static inline void mul_l1nl2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    Fec_rawMMul(r->longVal, a->longVal, b->longVal);
    Fec_rawMMul(r->longVal, r->longVal, Fec_R3.longVal);
}

static inline void mul_l1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;
    Fec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;
    Fec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;
    Fec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ns2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fec_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        Fec_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fec_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }

    Fec_rawMMul(r->longVal, r->longVal, Fec_R3.longVal);
}

static inline void mul_s1nl2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        Fec_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        Fec_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fec_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }

    Fec_rawMMul(r->longVal, r->longVal, Fec_R3.longVal);
}

static inline void mul_l1ms2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fec_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        Fec_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fec_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void mul_s1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        Fec_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        Fec_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fec_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }
}

static inline void mul_l1ns2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;
    Fec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ms2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;
    Fec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;
    Fec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;
    Fec_rawMMul(r->longVal, a->longVal, b->longVal);
}

void Fec_mul(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
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
                if (b->type & Fec_MONTGOMERY)
                {
                    mul_l1nl2m(r, a, b);
                }
                else
                {
                    mul_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            if (b->type & Fec_MONTGOMERY)
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
            if (b->type & Fec_MONTGOMERY)
            {
                mul_l1ns2m(r, a, b);
            }
            else
            {
                mul_l1ns2n(r, a, b);
            }
        }
    }
    else if (b->type & Fec_LONG)
    {
        if (a->type & Fec_MONTGOMERY)
        {
            if (b->type & Fec_MONTGOMERY)
            {
                mul_s1ml2m(r, a, b);
            }
            else
            {
                mul_s1ml2n(r,a, b);
            }
        }
        else if (b->type & Fec_MONTGOMERY)
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

void Fec_toLongNormal(PFecElement r, PFecElement a)
{
    if (a->type & Fec_LONG)
    {
        if (a->type & Fec_MONTGOMERY)
        {
            Fec_rawFromMontgomery(r->longVal, a->longVal);
            r->type = Fec_LONG;
        }
        else
        {
            Fec_copy(r, a);
        }
    }
    else
    {
        Fec_rawCopyS2L(r->longVal, a->shortVal);
        r->type = Fec_LONG;
        r->shortVal = 0;
    }
}

void Fec_toMontgomery(PFecElement r, PFecElement a)
{
    if (a->type & Fec_MONTGOMERY)
    {
        Fec_copy(r, a);
    }
    else if (a->type & Fec_LONG)
    {
        r->shortVal = a->shortVal;

        Fec_rawMMul(r->longVal, a->longVal, Fec_R2.longVal);

        r->type = Fec_LONGMONTGOMERY;
    }
    else if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
       Fec_rawMMul1(r->longVal, Fec_R2.longVal, -a_shortVal);
       Fec_rawNeg(r->longVal, r->longVal);

       r->type = Fec_SHORTMONTGOMERY;
    }
    else
    {
        Fec_rawMMul1(r->longVal, Fec_R2.longVal, a->shortVal);

        r->type = Fec_SHORTMONTGOMERY;
    }
}

void Fec_copyn(PFecElement r, PFecElement a, int n)
{
    std::memcpy(r, a, n * sizeof(FecElement));
}

static inline int has_add32_overflow(int64_t val)
{
    int64_t signs = (val >> 31) & 0x3;

    return signs == 1 || signs == 2;
}

static inline int Fec_rawSSub(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a - b;

    return has_add32_overflow(*r);
}

static inline void sub_s1s2(PFecElement r, PFecElement a, PFecElement b)
{
    int64_t diff;

    int overflow = Fec_rawSSub(&diff, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fec_rawCopyS2L(r->longVal, diff);
        r->type = Fec_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = Fec_SHORT;
        r->shortVal = (int32_t)diff;
    }
}

static inline void sub_l1nl2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    Fec_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    FecElement a_m;
    Fec_toMontgomery(&a_m, a);

    Fec_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    Fec_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1ml2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    FecElement b_m;
    Fec_toMontgomery(&b_m, b);

    Fec_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1l2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    if (a->shortVal >= 0)
    {
        Fec_rawSubSL(r->longVal, a->shortVal, b->longVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        Fec_rawNegLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void sub_l1ms2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    FecElement b_m;
    Fec_toMontgomery(&b_m, b);

    Fec_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    FecElement a_m;
    Fec_toMontgomery(&a_m, a);

    Fec_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ns2(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fec_rawAddLS(r->longVal, a->longVal, -b_shortVal);
    }
    else
    {
        Fec_rawSubLS(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void sub_l1ms2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    Fec_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_s1ml2m(PFecElement r,PFecElement a,PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    Fec_rawSub(r->longVal, a->longVal, b->longVal);
}

void Fec_sub(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
                {
                    sub_l1ml2m(r, a, b);
                }
                else
                {
                    sub_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fec_MONTGOMERY)
            {
                sub_l1nl2m(r, a, b);
            }
            else
            {
                sub_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            if (b->type & Fec_MONTGOMERY)
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
    else if (b->type & Fec_LONG)
    {
        if (b->type & Fec_MONTGOMERY)
        {
            if (a->type & Fec_MONTGOMERY)
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

static inline int Fec_rawSAdd(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a + b;

    return has_add32_overflow(*r);
}

static inline void add_s1s2(PFecElement r, PFecElement a, PFecElement b)
{
    int64_t sum;

    int overflow = Fec_rawSAdd(&sum, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fec_rawCopyS2L(r->longVal, sum);
        r->type = Fec_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = Fec_SHORT;
        r->shortVal = (int32_t)sum;
    }
}

static inline void add_l1nl2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    Fec_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    FecElement a_m;
    Fec_toMontgomery(&a_m, a);

    Fec_rawAdd(r->longVal, a_m.longVal, b->longVal);
}

static inline void add_l1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;
    Fec_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1ml2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    FecElement b_m;
    Fec_toMontgomery(&b_m, b);

    Fec_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1l2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    if (a->shortVal >= 0)
    {
        Fec_rawAddLS(r->longVal, b->longVal, a->shortVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        Fec_rawSubLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void add_l1ms2n(PFecElement r, PFecElement a, PFecElement b)
{
    FecElement b_m;

    r->type = Fec_LONGMONTGOMERY;

    Fec_toMontgomery(&b_m, b);

    Fec_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    FecElement m_a;
    Fec_toMontgomery(&m_a, a);

    Fec_rawAdd(r->longVal, m_a.longVal, b->longVal);
}

static inline void add_l1ns2(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    if (b->shortVal >= 0)
    {
        Fec_rawAddLS(r->longVal, a->longVal, b->shortVal);
    }
    else
    {
        int64_t b_shortVal = b->shortVal;
        Fec_rawSubLS(r->longVal, a->longVal, -b_shortVal);
    }
}

static inline void add_l1ms2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    Fec_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_s1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONGMONTGOMERY;

    Fec_rawAdd(r->longVal, a->longVal, b->longVal);
}

void Fec_add(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
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
                if (b->type & Fec_MONTGOMERY)
                {
                    add_l1nl2m(r, a, b);
                }
                else
                {
                    add_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            if (b->type & Fec_MONTGOMERY)
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
    else if (b->type & Fec_LONG)
    {
        if (b->type & Fec_MONTGOMERY)
        {
            if (a->type & Fec_MONTGOMERY)
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

int Fec_isTrue(PFecElement pE)
{
    int result;

    if (pE->type & Fec_LONG)
    {
        result = !Fec_rawIsZero(pE->longVal);
    }
    else
    {
        result = pE->shortVal != 0;
    }

    return result;
}

int Fec_longNeg(PFecElement pE)
{
    if(Fec_rawCmp(pE->longVal, Fec_q.longVal) >= 0)
    {
       Fec_longErr();
       return 0;
    }

    int64_t result = pE->longVal[0] - Fec_q.longVal[0];

    int64_t is_long = (result >> 31) + 1;

    if(is_long)
    {
       Fec_longErr();
       return 0;
    }

    return result;
}

int Fec_longNormal(PFecElement pE)
{
    uint64_t is_long = 0;
    uint64_t result;

    result = pE->longVal[0];

    is_long = result >> 31;

    if (is_long)
    {
         return Fec_longNeg(pE);
    }

    if (memcmp(&pE->longVal[1], zero, (sizeof(pE->longVal) - sizeof(pE->longVal[0]))))
    {
        return Fec_longNeg(pE);
    }

    return result;
}

// Convert a 64 bit integer to a long format field element
int Fec_toInt(PFecElement pE)
{
    int result;

    if (pE->type & Fec_LONG)
    {
       if (pE->type & Fec_MONTGOMERY)
       {
           FecElement e_n;
           Fec_toNormal(&e_n, pE);

           result = Fec_longNormal(&e_n);
       }
       else
       {
           result = Fec_longNormal(pE);
       }
    }
    else
    {
        result = pE->shortVal;
    }

    return result;
}

static inline int rlt_s1s2(PFecElement a, PFecElement b)
{
    return (a->shortVal < b->shortVal) ? 1 : 0;
}

static inline int rltRawL1L2(FecRawElement pRawA, FecRawElement pRawB)
{
    int result = Fec_rawCmp(pRawB, pRawA);

    return result > 0 ? 1 : 0;
}

static inline int rltl1l2_n1(FecRawElement pRawA, FecRawElement pRawB)
{
    int result = Fec_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rltRawL1L2(pRawA, pRawB);
    }

     return 1;
}

static inline int rltl1l2_p1(FecRawElement pRawA, FecRawElement pRawB)
{
    int result = Fec_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 0;
    }

    return rltRawL1L2(pRawA, pRawB);
}

static inline int rltL1L2(FecRawElement pRawA, FecRawElement pRawB)
{
    int result = Fec_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rltl1l2_n1(pRawA, pRawB);
    }

    return rltl1l2_p1(pRawA, pRawB);
}

static inline int rlt_l1nl2n(PFecElement a, PFecElement b)
{
    return rltL1L2(a->longVal, b->longVal);
}

static inline int rlt_l1nl2m(PFecElement a, PFecElement b)
{
    FecElement b_n;

    Fec_toNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

static inline int rlt_l1ml2m(PFecElement a, PFecElement b)
{
    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);
    Fec_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ml2n(PFecElement a, PFecElement b)
{
    FecElement a_n;

    Fec_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_s1l2n(PFecElement a,PFecElement b)
{
    FecElement a_n;

    Fec_toLongNormal(&a_n,a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_l1ms2(PFecElement a, PFecElement b)
{
    FecElement a_n;
    FecElement b_ln;

    Fec_toLongNormal(&b_ln ,b);
    Fec_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b_ln.longVal);
}

static inline int rlt_s1l2m(PFecElement a, PFecElement b)
{
    FecElement a_n;
    FecElement b_n;

    Fec_toLongNormal(&a_n, a);
    Fec_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ns2(PFecElement a, PFecElement b)
{
    FecElement b_n;

    Fec_toLongNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

int32_t Fec_rlt(PFecElement a, PFecElement b)
{
    int32_t result;

    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
                {
                    result = rlt_l1ml2m(a, b);
                }
                else
                {
                    result = rlt_l1ml2n(a, b);
                }
            }
            else if (b->type & Fec_MONTGOMERY)
            {
                result = rlt_l1nl2m(a, b);
            }
            else
            {
                result = rlt_l1nl2n(a, b);
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            result = rlt_l1ms2(a, b);
        }
        else
        {
            result = rlt_l1ns2(a, b);
        }
    }
    else if (b->type & Fec_LONG)
    {
        if (b->type & Fec_MONTGOMERY)
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

void Fec_lt(PFecElement r, PFecElement a, PFecElement b)
{
    r->shortVal = Fec_rlt(a, b);
    r->type = Fec_SHORT;
}

void Fec_geq(PFecElement r, PFecElement a, PFecElement b)
{
   int32_t result = Fec_rlt(a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = Fec_SHORT;
}

static inline int Fec_rawSNeg(int64_t *r, int32_t a)
{
    *r = -(int64_t)a;

    return has_add32_overflow(*r);
}

void Fec_neg(PFecElement r, PFecElement a)
{
    if (a->type & Fec_LONG)
    {
        r->type = a->type;
        r->shortVal = a->shortVal;
        Fec_rawNeg(r->longVal, a->longVal);
    }
    else
    {
        int64_t a_shortVal;

        int overflow = Fec_rawSNeg(&a_shortVal, a->shortVal);

        if (overflow)
        {
            Fec_rawCopyS2L(r->longVal, a_shortVal);
            r->type = Fec_LONG;
            r->shortVal = 0;
        }
        else
        {
            r->type = Fec_SHORT;
            r->shortVal = (int32_t)a_shortVal;
        }
    }
}

static inline int reqL1L2(FecRawElement pRawA, FecRawElement pRawB)
{
    return Fec_rawCmp(pRawB, pRawA) == 0;
}

static inline int req_s1s2(PFecElement r, PFecElement a, PFecElement b)
{
    return (a->shortVal == b->shortVal) ? 1 : 0;
}

static inline int req_l1nl2n(PFecElement r, PFecElement a, PFecElement b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    FecElement a_m;
    Fec_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1ml2n(PFecElement r, PFecElement a, PFecElement b)
{
    FecElement b_m;
    Fec_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2n(PFecElement r, PFecElement a, PFecElement b)
{
    FecElement a_n;
    Fec_toLongNormal(&a_n, a);

    return reqL1L2(a_n.longVal, b->longVal);
}

static inline int req_l1ms2(PFecElement r, PFecElement a, PFecElement b)
{
    FecElement b_m;
    Fec_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2m(PFecElement r, PFecElement a, PFecElement b)
{
    FecElement a_m;
    Fec_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ns2(PFecElement r, PFecElement a, PFecElement b)
{
    FecElement b_n;
    Fec_toLongNormal(&b_n, b);

    return reqL1L2(a->longVal, b_n.longVal);
}

// Compares two elements of any kind
int Fec_req(PFecElement r, PFecElement a, PFecElement b)
{
    int result;

    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
                {
                    result = req_l1ml2m(r, a, b);
                }
                else
                {
                    result = req_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fec_MONTGOMERY)
            {
                result = req_l1nl2m(r, a, b);
            }
            else
            {
                result = req_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            result = req_l1ms2(r, a, b);
        }
        else
        {
            result = req_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fec_LONG)
    {
        if (b->type & Fec_MONTGOMERY)
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

void Fec_eq(PFecElement r, PFecElement a, PFecElement b)
{
    r->shortVal = Fec_req(r, a, b);
    r->type = Fec_SHORT;
}

void Fec_neq(PFecElement r, PFecElement a, PFecElement b)
{
    int result = Fec_req(r, a, b);

    r->shortVal = result ^ 0x1;
    r->type = Fec_SHORT;
}

// Logical or between two elements
void Fec_lor(PFecElement r, PFecElement a, PFecElement b)
{
    int32_t is_true_a;

    if (a->type & Fec_LONG)
    {
        is_true_a = !Fec_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & Fec_LONG)
    {
        is_true_b = !Fec_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a | is_true_b;
    r->type = Fec_SHORT;
}

void Fec_lnot(PFecElement r, PFecElement a)
{
    if (a->type & Fec_LONG)
    {
        r->shortVal = Fec_rawIsZero(a->longVal);
    }
    else
    {
        r->shortVal = a->shortVal ? 0 : 1;
    }

    r->type = Fec_SHORT;
}


static inline int rgt_s1s2(PFecElement a, PFecElement b)
{
    return (a->shortVal > b->shortVal) ? 1 : 0;
}

static inline int rgtRawL1L2(FecRawElement pRawA, FecRawElement pRawB)
{
    int result = Fec_rawCmp(pRawB, pRawA);

    return (result < 0) ? 1 : 0;
}

static inline int rgtl1l2_n1(FecRawElement pRawA, FecRawElement pRawB)
{
    int result = Fec_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rgtRawL1L2(pRawA, pRawB);
    }
    return 0;
}

static inline int rgtl1l2_p1(FecRawElement pRawA, FecRawElement pRawB)
{
    int result = Fec_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 1;
    }
    return rgtRawL1L2(pRawA, pRawB);
}

static inline int rgtL1L2(FecRawElement pRawA, FecRawElement pRawB)
{
    int result = Fec_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rgtl1l2_n1(pRawA, pRawB);
    }

    return rgtl1l2_p1(pRawA, pRawB);
}

static inline int rgt_l1nl2n(PFecElement a, PFecElement b)
{
    return rgtL1L2(a->longVal, b->longVal);
}

static inline int rgt_l1nl2m(PFecElement a, PFecElement b)
{
    FecElement b_n;
    Fec_toNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

static inline int rgt_l1ml2m(PFecElement a, PFecElement b)
{
    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);
    Fec_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ml2n(PFecElement a, PFecElement b)
{
    FecElement a_n;
    Fec_toNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_s1l2n(PFecElement a, PFecElement b)
{
    FecElement a_n;
    Fec_toLongNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_l1ms2(PFecElement a, PFecElement b)
{
    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);
    Fec_toLongNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_s1l2m(PFecElement a, PFecElement b)
{
    FecElement a_n;
    FecElement b_n;

    Fec_toLongNormal(&a_n, a);
    Fec_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ns2(PFecElement a, PFecElement b)
{
    FecElement b_n;
    Fec_toLongNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

int Fec_rgt(PFecElement r, PFecElement a, PFecElement b)
{
    int result = 0;

    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
                {
                    result = rgt_l1ml2m(a, b);
                }
                else
                {
                    result = rgt_l1ml2n(a, b);
                }
            }
            else if (b->type & Fec_MONTGOMERY)
            {
                result = rgt_l1nl2m(a, b);
            }
            else
            {
                result = rgt_l1nl2n(a, b);
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            result = rgt_l1ms2(a, b);
        }
        else
        {
            result = rgt_l1ns2(a, b);
        }
    }
    else if (b->type & Fec_LONG)
    {
        if (b->type & Fec_MONTGOMERY)
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

void Fec_gt(PFecElement r, PFecElement a, PFecElement b)
{
    r->shortVal = Fec_rgt(r, a, b);
    r->type = Fec_SHORT;
}

void Fec_leq(PFecElement r, PFecElement a, PFecElement b)
{
   int32_t result = Fec_rgt(r, a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = Fec_SHORT;
}

// Logical and between two elements
void Fec_land(PFecElement r, PFecElement a, PFecElement b)
{
    int32_t is_true_a;

    if (a->type & Fec_LONG)
    {
        is_true_a = !Fec_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & Fec_LONG)
    {
        is_true_b = !Fec_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a & is_true_b;
    r->type = Fec_SHORT;
}

static inline void and_s1s2(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        int32_t result = a->shortVal & b->shortVal;
        r->shortVal = result;
        r->type = Fec_SHORT;
        return;
    }

    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toLongNormal(&a_n, a);
    Fec_toLongNormal(&b_n, b);

    Fec_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1nl2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;
    Fec_rawAnd(r->longVal, a->longVal, b->longVal);
}

static inline void and_l1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement b_n;
    Fec_toNormal(&b_n, b);

    Fec_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

static inline void and_l1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);
    Fec_toNormal(&b_n, b);

    Fec_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ml2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    Fec_toNormal(&a_n, a);

    Fec_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_s1l2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&a_n, a);
    }

    Fec_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_l1ms2(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&b_n, b);
    }

    Fec_rawAnd(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void and_s1l2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&a_n, a);
    }

    Fec_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ns2(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&b_n, b);
    }

    Fec_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

// Ands two elements of any kind
void Fec_band(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
                {
                    and_l1ml2m(r, a, b);
                }
                else
                {
                    and_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fec_MONTGOMERY)
            {
                and_l1nl2m(r, a, b);
            }
            else
            {
                and_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            and_l1ms2(r, a, b);
        }
        else
        {
           and_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fec_LONG)
    {
        if (b->type & Fec_MONTGOMERY)
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

void Fec_rawZero(FecRawElement pRawResult)
{
    std::memset(pRawResult, 0, sizeof(FecRawElement));
}

static inline void rawShl(FecRawElement r, FecRawElement a, uint64_t b)
{
    if (b == 0)
    {
        Fec_rawCopy(r, a);
        return;
    }

    if (b >= 256)
    {
        Fec_rawZero(r);
        return;
    }

    Fec_rawShl(r, a, b);
}

static inline void rawShr(FecRawElement r, FecRawElement a, uint64_t b)
{
    if (b == 0)
    {
        Fec_rawCopy(r, a);
        return;
    }

    if (b >= 256)
    {
        Fec_rawZero(r);
        return;
    }

    Fec_rawShr(r,a, b);
}

static inline void Fec_setzero(PFecElement r)
{
    r->type = 0;
    r->shortVal = 0;
}

static inline void do_shlcl(PFecElement r, PFecElement a, uint64_t b)
{
    FecElement a_long;
    Fec_toLongNormal(&a_long, a);

    r->type = Fec_LONG;
    rawShl(r->longVal, a_long.longVal, b);
}

static inline void do_shlln(PFecElement r, PFecElement a, uint64_t b)
{
    r->type = Fec_LONG;
    rawShl(r->longVal, a->longVal, b);
}

static inline void do_shl(PFecElement r, PFecElement a, uint64_t b)
{
    if (a->type & Fec_LONG)
    {
        if (a->type == Fec_LONGMONTGOMERY)
        {
            FecElement a_long;
            Fec_toNormal(&a_long, a);

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
            Fec_setzero(r);
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
                r->type = Fec_SHORT;
                r->shortVal = a_shortVal;
            }
        }
    }
}

static inline void do_shrln(PFecElement r, PFecElement a, uint64_t b)
{
    r->type = Fec_LONG;
    rawShr(r->longVal, a->longVal, b);
}

static inline void do_shrl(PFecElement r, PFecElement a, uint64_t b)
{
    if (a->type == Fec_LONGMONTGOMERY)
    {
        FecElement a_long;
        Fec_toNormal(&a_long, a);

        do_shrln(r, &a_long, b);
    }
    else
    {
        do_shrln(r, a, b);
    }
}

static inline void do_shr(PFecElement r, PFecElement a, uint64_t b)
{
    if (a->type & Fec_LONG)
    {
        do_shrl(r, a, b);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            Fec_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            FecElement a_long;
            Fec_toLongNormal(&a_long, a);

            do_shrl(r, &a_long, b);
        }
        else if(b >= 31)
        {
            Fec_setzero(r);
        }
        else
        {
            a_shortVal >>= b;

            r->shortVal = a_shortVal;
            r->type = Fec_SHORT;
        }
    }
}

static inline void Fec_shr_big_shift(PFecElement r, PFecElement a, PFecElement b)
{
    static FecRawElement max_shift = {256};

    FecRawElement shift;

    Fec_rawSubRegular(shift, Fec_q.longVal, b->longVal);

    if (Fec_rawCmp(shift, max_shift) >= 0)
    {
        Fec_setzero(r);
    }
    else
    {
        do_shl(r, a, shift[0]);
    }
}

static inline void Fec_shr_long(PFecElement r, PFecElement a, PFecElement b)
{
    static FecRawElement max_shift = {256};

    if (Fec_rawCmp(b->longVal, max_shift) >= 0)
    {
        Fec_shr_big_shift(r, a, b);
    }
    else
    {
        do_shr(r, a, b->longVal[0]);
    }
}

void Fec_shr(PFecElement r, PFecElement a, PFecElement b)
{
    if (b->type & Fec_LONG)
    {
        if (b->type == Fec_LONGMONTGOMERY)
        {
            FecElement b_long;
            Fec_toNormal(&b_long, b);

            Fec_shr_long(r, a, &b_long);
        }
        else
        {
            Fec_shr_long(r, a, b);
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
                Fec_setzero(r);
            }
            else
            {
                do_shl(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 256)
        {
            Fec_setzero(r);
        }
        else
        {
            do_shr(r, a, b_shortVal);
        }
    }
}

static inline void Fec_shl_big_shift(PFecElement r, PFecElement a, PFecElement b)
{
    static FecRawElement max_shift = {256};

    FecRawElement shift;

    Fec_rawSubRegular(shift, Fec_q.longVal, b->longVal);

    if (Fec_rawCmp(shift, max_shift) >= 0)
    {
        Fec_setzero(r);
    }
    else
    {
        do_shr(r, a, shift[0]);
    }
}

static inline void Fec_shl_long(PFecElement r, PFecElement a, PFecElement b)
{
    static FecRawElement max_shift = {256};

    if (Fec_rawCmp(b->longVal, max_shift) >= 0)
    {
        Fec_shl_big_shift(r, a, b);
    }
    else
    {
        do_shl(r, a, b->longVal[0]);
    }
}

void Fec_shl(PFecElement r, PFecElement a, PFecElement b)
{
    if (b->type & Fec_LONG)
    {
        if (b->type == Fec_LONGMONTGOMERY)
        {
            FecElement b_long;
            Fec_toNormal(&b_long, b);

            Fec_shl_long(r, a, &b_long);
        }
        else
        {
            Fec_shl_long(r, a, b);
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
                Fec_setzero(r);
            }
            else
            {
                do_shr(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 256)
        {
            Fec_setzero(r);
        }
        else
        {
            do_shl(r, a, b_shortVal);
        }
    }
}

void Fec_square(PFecElement r, PFecElement a)
{
    if (a->type & Fec_LONG)
    {
        if (a->type == Fec_LONGMONTGOMERY)
        {
            r->type = Fec_LONGMONTGOMERY;
            Fec_rawMSquare(r->longVal, a->longVal);
        }
        else
        {
            r->type = Fec_LONGMONTGOMERY;
            Fec_rawMSquare(r->longVal, a->longVal);
            Fec_rawMMul(r->longVal, r->longVal, Fec_R3.longVal);
        }
    }
    else
    {
        int64_t result;

        int overflow = Fec_rawSMul(&result, a->shortVal, a->shortVal);

        if (overflow)
        {
            Fec_rawCopyS2L(r->longVal, result);
            r->type = Fec_LONG;
            r->shortVal = 0;
        }
        else
        {
            // done the same way as in intel asm implementation
            r->shortVal = (int32_t)result;
            r->type = Fec_SHORT;
            //

            Fec_rawCopyS2L(r->longVal, result);
            r->type = Fec_LONG;
            r->shortVal = 0;
        }
    }
}

static inline void or_s1s2(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal | b->shortVal;
        r->type = Fec_SHORT;
        return;
    }

    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toLongNormal(&a_n, a);
    Fec_toLongNormal(&b_n, b);

    Fec_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&a_n, a);
    }

    Fec_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&a_n, a);
    }

    Fec_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ns2(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&b_n, b);
    }

    Fec_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ms2(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&b_n, b);
    }

    Fec_rawOr(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void or_l1nl2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;
    Fec_rawOr(r->longVal, a->longVal, b->longVal);
}

static inline void or_l1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement b_n;
    Fec_toNormal(&b_n, b);

    Fec_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ml2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    Fec_toNormal(&a_n, a);

    Fec_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);
    Fec_toNormal(&b_n, b);

    Fec_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}


void Fec_bor(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
                {
                    or_l1ml2m(r, a, b);
                }
                else
                {
                    or_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fec_MONTGOMERY)
            {
                or_l1nl2m(r, a, b);
            }
            else
            {
                or_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            or_l1ms2(r, a, b);
        }
        else
        {
           or_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fec_LONG)
    {
        if (b->type & Fec_MONTGOMERY)
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

static inline void xor_s1s2(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal ^ b->shortVal;
        r->type = Fec_SHORT;
        return;
    }

    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toLongNormal(&a_n, a);
    Fec_toLongNormal(&b_n, b);

    Fec_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_s1l2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&a_n, a);
    }

    Fec_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_s1l2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&a_n, a);
    }

    Fec_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_l1ns2(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&b_n, b);
    }

    Fec_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ms2(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fec_toLongNormal(&b_n, b);
    }

    Fec_rawXor(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void xor_l1nl2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;
    Fec_rawXor(r->longVal, a->longVal, b->longVal);
}

static inline void xor_l1nl2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement b_n;
    Fec_toNormal(&b_n, b);

    Fec_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ml2n(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    Fec_toNormal(&a_n, a);

    Fec_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_l1ml2m(PFecElement r, PFecElement a, PFecElement b)
{
    r->type = Fec_LONG;

    FecElement a_n;
    FecElement b_n;

    Fec_toNormal(&a_n, a);
    Fec_toNormal(&b_n, b);

    Fec_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

void Fec_bxor(PFecElement r, PFecElement a, PFecElement b)
{
    if (a->type & Fec_LONG)
    {
        if (b->type & Fec_LONG)
        {
            if (a->type & Fec_MONTGOMERY)
            {
                if (b->type & Fec_MONTGOMERY)
                {
                    xor_l1ml2m(r, a, b);
                }
                else
                {
                    xor_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fec_MONTGOMERY)
            {
                xor_l1nl2m(r, a, b);
            }
            else
            {
                xor_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fec_MONTGOMERY)
        {
            xor_l1ms2(r, a, b);
        }
        else
        {
           xor_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fec_LONG)
    {
        if (b->type & Fec_MONTGOMERY)
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

void Fec_bnot(PFecElement r, PFecElement a)
{
    r->type = Fec_LONG;

    if (a->type == Fec_LONG)
    {
        if (a->type & Fec_MONTGOMERY)
        {
            FecElement a_n;
            Fec_toNormal(&a_n, a);

            Fec_rawNot(r->longVal, a_n.longVal);
        }
        else
        {
            Fec_rawNot(r->longVal, a->longVal);
        }
    }
    else
    {
        FecElement a_n;
        Fec_toLongNormal(&a_n, a);

        Fec_rawNot(r->longVal, a_n.longVal);
    }
}
