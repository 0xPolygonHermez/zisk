#include "fnec.hpp"
#include <cstdint>
#include <cstring>
#include <cassert>

FnecElement Fnec_q  = {0, 0x80000000, {0xbfd25e8cd0364141,0xbaaedce6af48a03b,0xfffffffffffffffe,0xffffffffffffffff}};
FnecElement Fnec_R2 = {0, 0x80000000, {0x896cf21467d7d140,0x741496c20e7cf878,0xe697f5e45bcd07c6,0x9d671cd581c69bc5}};
FnecElement Fnec_R3 = {0, 0x80000000, {0x7bc0cfe0e9ff41ed,0x0017648444d4322c,0xb1b31347f1d0b2da,0x555d800c18ef116d}};

static FnecRawElement half = {0xdfe92f46681b20a0,0x5d576e7357a4501d,0xffffffffffffffff,0x7fffffffffffffff};
static FnecRawElement zero = {0};


void Fnec_copy(PFnecElement r, const PFnecElement a)
{
    *r = *a;
}

void Fnec_toNormal(PFnecElement r, PFnecElement a)
{
    if (a->type == Fnec_LONGMONTGOMERY)
    {
        r->type = Fnec_LONG;
        Fnec_rawFromMontgomery(r->longVal, a->longVal);
    }
    else
    {
        Fnec_copy(r, a);
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

static inline int Fnec_rawSMul(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a * b;

    return has_mul32_overflow(*r);
}

static inline void mul_s1s2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    int64_t result;

    int overflow = Fnec_rawSMul(&result, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fnec_rawCopyS2L(r->longVal, result);
        r->type = Fnec_LONG;
        r->shortVal = 0;
    }
    else
    {
        // done the same way as in intel asm implementation
        r->shortVal = (int32_t)result;
        r->type = Fnec_SHORT;
        //

        Fnec_rawCopyS2L(r->longVal, result);
        r->type = Fnec_LONG;
        r->shortVal = 0;
    }
}

static inline void mul_l1nl2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    Fnec_rawMMul(r->longVal, a->longVal, b->longVal);
    Fnec_rawMMul(r->longVal, r->longVal, Fnec_R3.longVal);
}

static inline void mul_l1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;
    Fnec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;
    Fnec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;
    Fnec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ns2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fnec_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        Fnec_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fnec_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }

    Fnec_rawMMul(r->longVal, r->longVal, Fnec_R3.longVal);
}

static inline void mul_s1nl2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        Fnec_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        Fnec_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fnec_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }

    Fnec_rawMMul(r->longVal, r->longVal, Fnec_R3.longVal);
}

static inline void mul_l1ms2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fnec_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        Fnec_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fnec_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void mul_s1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        Fnec_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        Fnec_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fnec_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }
}

static inline void mul_l1ns2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;
    Fnec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ms2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;
    Fnec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;
    Fnec_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;
    Fnec_rawMMul(r->longVal, a->longVal, b->longVal);
}

void Fnec_mul(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
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
                if (b->type & Fnec_MONTGOMERY)
                {
                    mul_l1nl2m(r, a, b);
                }
                else
                {
                    mul_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            if (b->type & Fnec_MONTGOMERY)
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
            if (b->type & Fnec_MONTGOMERY)
            {
                mul_l1ns2m(r, a, b);
            }
            else
            {
                mul_l1ns2n(r, a, b);
            }
        }
    }
    else if (b->type & Fnec_LONG)
    {
        if (a->type & Fnec_MONTGOMERY)
        {
            if (b->type & Fnec_MONTGOMERY)
            {
                mul_s1ml2m(r, a, b);
            }
            else
            {
                mul_s1ml2n(r,a, b);
            }
        }
        else if (b->type & Fnec_MONTGOMERY)
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

void Fnec_toLongNormal(PFnecElement r, PFnecElement a)
{
    if (a->type & Fnec_LONG)
    {
        if (a->type & Fnec_MONTGOMERY)
        {
            Fnec_rawFromMontgomery(r->longVal, a->longVal);
            r->type = Fnec_LONG;
        }
        else
        {
            Fnec_copy(r, a);
        }
    }
    else
    {
        Fnec_rawCopyS2L(r->longVal, a->shortVal);
        r->type = Fnec_LONG;
        r->shortVal = 0;
    }
}

void Fnec_toMontgomery(PFnecElement r, PFnecElement a)
{
    if (a->type & Fnec_MONTGOMERY)
    {
        Fnec_copy(r, a);
    }
    else if (a->type & Fnec_LONG)
    {
        r->shortVal = a->shortVal;

        Fnec_rawMMul(r->longVal, a->longVal, Fnec_R2.longVal);

        r->type = Fnec_LONGMONTGOMERY;
    }
    else if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
       Fnec_rawMMul1(r->longVal, Fnec_R2.longVal, -a_shortVal);
       Fnec_rawNeg(r->longVal, r->longVal);

       r->type = Fnec_SHORTMONTGOMERY;
    }
    else
    {
        Fnec_rawMMul1(r->longVal, Fnec_R2.longVal, a->shortVal);

        r->type = Fnec_SHORTMONTGOMERY;
    }
}

void Fnec_copyn(PFnecElement r, PFnecElement a, int n)
{
    std::memcpy(r, a, n * sizeof(FnecElement));
}

static inline int has_add32_overflow(int64_t val)
{
    int64_t signs = (val >> 31) & 0x3;

    return signs == 1 || signs == 2;
}

static inline int Fnec_rawSSub(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a - b;

    return has_add32_overflow(*r);
}

static inline void sub_s1s2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    int64_t diff;

    int overflow = Fnec_rawSSub(&diff, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fnec_rawCopyS2L(r->longVal, diff);
        r->type = Fnec_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = Fnec_SHORT;
        r->shortVal = (int32_t)diff;
    }
}

static inline void sub_l1nl2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    Fnec_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    FnecElement a_m;
    Fnec_toMontgomery(&a_m, a);

    Fnec_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    Fnec_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1ml2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    FnecElement b_m;
    Fnec_toMontgomery(&b_m, b);

    Fnec_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1l2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    if (a->shortVal >= 0)
    {
        Fnec_rawSubSL(r->longVal, a->shortVal, b->longVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        Fnec_rawNegLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void sub_l1ms2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    FnecElement b_m;
    Fnec_toMontgomery(&b_m, b);

    Fnec_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    FnecElement a_m;
    Fnec_toMontgomery(&a_m, a);

    Fnec_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ns2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fnec_rawAddLS(r->longVal, a->longVal, -b_shortVal);
    }
    else
    {
        Fnec_rawSubLS(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void sub_l1ms2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    Fnec_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_s1ml2m(PFnecElement r,PFnecElement a,PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    Fnec_rawSub(r->longVal, a->longVal, b->longVal);
}

void Fnec_sub(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
                {
                    sub_l1ml2m(r, a, b);
                }
                else
                {
                    sub_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fnec_MONTGOMERY)
            {
                sub_l1nl2m(r, a, b);
            }
            else
            {
                sub_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            if (b->type & Fnec_MONTGOMERY)
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
    else if (b->type & Fnec_LONG)
    {
        if (b->type & Fnec_MONTGOMERY)
        {
            if (a->type & Fnec_MONTGOMERY)
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

static inline int Fnec_rawSAdd(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a + b;

    return has_add32_overflow(*r);
}

static inline void add_s1s2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    int64_t sum;

    int overflow = Fnec_rawSAdd(&sum, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fnec_rawCopyS2L(r->longVal, sum);
        r->type = Fnec_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = Fnec_SHORT;
        r->shortVal = (int32_t)sum;
    }
}

static inline void add_l1nl2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    Fnec_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    FnecElement a_m;
    Fnec_toMontgomery(&a_m, a);

    Fnec_rawAdd(r->longVal, a_m.longVal, b->longVal);
}

static inline void add_l1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;
    Fnec_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1ml2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    FnecElement b_m;
    Fnec_toMontgomery(&b_m, b);

    Fnec_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1l2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    if (a->shortVal >= 0)
    {
        Fnec_rawAddLS(r->longVal, b->longVal, a->shortVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        Fnec_rawSubLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void add_l1ms2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    FnecElement b_m;

    r->type = Fnec_LONGMONTGOMERY;

    Fnec_toMontgomery(&b_m, b);

    Fnec_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    FnecElement m_a;
    Fnec_toMontgomery(&m_a, a);

    Fnec_rawAdd(r->longVal, m_a.longVal, b->longVal);
}

static inline void add_l1ns2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    if (b->shortVal >= 0)
    {
        Fnec_rawAddLS(r->longVal, a->longVal, b->shortVal);
    }
    else
    {
        int64_t b_shortVal = b->shortVal;
        Fnec_rawSubLS(r->longVal, a->longVal, -b_shortVal);
    }
}

static inline void add_l1ms2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    Fnec_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_s1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONGMONTGOMERY;

    Fnec_rawAdd(r->longVal, a->longVal, b->longVal);
}

void Fnec_add(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
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
                if (b->type & Fnec_MONTGOMERY)
                {
                    add_l1nl2m(r, a, b);
                }
                else
                {
                    add_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            if (b->type & Fnec_MONTGOMERY)
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
    else if (b->type & Fnec_LONG)
    {
        if (b->type & Fnec_MONTGOMERY)
        {
            if (a->type & Fnec_MONTGOMERY)
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

int Fnec_isTrue(PFnecElement pE)
{
    int result;

    if (pE->type & Fnec_LONG)
    {
        result = !Fnec_rawIsZero(pE->longVal);
    }
    else
    {
        result = pE->shortVal != 0;
    }

    return result;
}

int Fnec_longNeg(PFnecElement pE)
{
    if(Fnec_rawCmp(pE->longVal, Fnec_q.longVal) >= 0)
    {
       Fnec_longErr();
       return 0;
    }

    int64_t result = pE->longVal[0] - Fnec_q.longVal[0];

    int64_t is_long = (result >> 31) + 1;

    if(is_long)
    {
       Fnec_longErr();
       return 0;
    }

    return result;
}

int Fnec_longNormal(PFnecElement pE)
{
    uint64_t is_long = 0;
    uint64_t result;

    result = pE->longVal[0];

    is_long = result >> 31;

    if (is_long)
    {
         return Fnec_longNeg(pE);
    }

    if (memcmp(&pE->longVal[1], zero, (sizeof(pE->longVal) - sizeof(pE->longVal[0]))))
    {
        return Fnec_longNeg(pE);
    }

    return result;
}

// Convert a 64 bit integer to a long format field element
int Fnec_toInt(PFnecElement pE)
{
    int result;

    if (pE->type & Fnec_LONG)
    {
       if (pE->type & Fnec_MONTGOMERY)
       {
           FnecElement e_n;
           Fnec_toNormal(&e_n, pE);

           result = Fnec_longNormal(&e_n);
       }
       else
       {
           result = Fnec_longNormal(pE);
       }
    }
    else
    {
        result = pE->shortVal;
    }

    return result;
}

static inline int rlt_s1s2(PFnecElement a, PFnecElement b)
{
    return (a->shortVal < b->shortVal) ? 1 : 0;
}

static inline int rltRawL1L2(FnecRawElement pRawA, FnecRawElement pRawB)
{
    int result = Fnec_rawCmp(pRawB, pRawA);

    return result > 0 ? 1 : 0;
}

static inline int rltl1l2_n1(FnecRawElement pRawA, FnecRawElement pRawB)
{
    int result = Fnec_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rltRawL1L2(pRawA, pRawB);
    }

     return 1;
}

static inline int rltl1l2_p1(FnecRawElement pRawA, FnecRawElement pRawB)
{
    int result = Fnec_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 0;
    }

    return rltRawL1L2(pRawA, pRawB);
}

static inline int rltL1L2(FnecRawElement pRawA, FnecRawElement pRawB)
{
    int result = Fnec_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rltl1l2_n1(pRawA, pRawB);
    }

    return rltl1l2_p1(pRawA, pRawB);
}

static inline int rlt_l1nl2n(PFnecElement a, PFnecElement b)
{
    return rltL1L2(a->longVal, b->longVal);
}

static inline int rlt_l1nl2m(PFnecElement a, PFnecElement b)
{
    FnecElement b_n;

    Fnec_toNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

static inline int rlt_l1ml2m(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);
    Fnec_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ml2n(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;

    Fnec_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_s1l2n(PFnecElement a,PFnecElement b)
{
    FnecElement a_n;

    Fnec_toLongNormal(&a_n,a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_l1ms2(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    FnecElement b_ln;

    Fnec_toLongNormal(&b_ln ,b);
    Fnec_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b_ln.longVal);
}

static inline int rlt_s1l2m(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    FnecElement b_n;

    Fnec_toLongNormal(&a_n, a);
    Fnec_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ns2(PFnecElement a, PFnecElement b)
{
    FnecElement b_n;

    Fnec_toLongNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

int32_t Fnec_rlt(PFnecElement a, PFnecElement b)
{
    int32_t result;

    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
                {
                    result = rlt_l1ml2m(a, b);
                }
                else
                {
                    result = rlt_l1ml2n(a, b);
                }
            }
            else if (b->type & Fnec_MONTGOMERY)
            {
                result = rlt_l1nl2m(a, b);
            }
            else
            {
                result = rlt_l1nl2n(a, b);
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            result = rlt_l1ms2(a, b);
        }
        else
        {
            result = rlt_l1ns2(a, b);
        }
    }
    else if (b->type & Fnec_LONG)
    {
        if (b->type & Fnec_MONTGOMERY)
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

void Fnec_lt(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->shortVal = Fnec_rlt(a, b);
    r->type = Fnec_SHORT;
}

void Fnec_geq(PFnecElement r, PFnecElement a, PFnecElement b)
{
   int32_t result = Fnec_rlt(a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = Fnec_SHORT;
}

static inline int Fnec_rawSNeg(int64_t *r, int32_t a)
{
    *r = -(int64_t)a;

    return has_add32_overflow(*r);
}

void Fnec_neg(PFnecElement r, PFnecElement a)
{
    if (a->type & Fnec_LONG)
    {
        r->type = a->type;
        r->shortVal = a->shortVal;
        Fnec_rawNeg(r->longVal, a->longVal);
    }
    else
    {
        int64_t a_shortVal;

        int overflow = Fnec_rawSNeg(&a_shortVal, a->shortVal);

        if (overflow)
        {
            Fnec_rawCopyS2L(r->longVal, a_shortVal);
            r->type = Fnec_LONG;
            r->shortVal = 0;
        }
        else
        {
            r->type = Fnec_SHORT;
            r->shortVal = (int32_t)a_shortVal;
        }
    }
}

static inline int reqL1L2(FnecRawElement pRawA, FnecRawElement pRawB)
{
    return Fnec_rawCmp(pRawB, pRawA) == 0;
}

static inline int req_s1s2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    return (a->shortVal == b->shortVal) ? 1 : 0;
}

static inline int req_l1nl2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    FnecElement a_m;
    Fnec_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1ml2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    FnecElement b_m;
    Fnec_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    Fnec_toLongNormal(&a_n, a);

    return reqL1L2(a_n.longVal, b->longVal);
}

static inline int req_l1ms2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    FnecElement b_m;
    Fnec_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    FnecElement a_m;
    Fnec_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ns2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    FnecElement b_n;
    Fnec_toLongNormal(&b_n, b);

    return reqL1L2(a->longVal, b_n.longVal);
}

// Compares two elements of any kind
int Fnec_req(PFnecElement r, PFnecElement a, PFnecElement b)
{
    int result;

    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
                {
                    result = req_l1ml2m(r, a, b);
                }
                else
                {
                    result = req_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fnec_MONTGOMERY)
            {
                result = req_l1nl2m(r, a, b);
            }
            else
            {
                result = req_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            result = req_l1ms2(r, a, b);
        }
        else
        {
            result = req_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fnec_LONG)
    {
        if (b->type & Fnec_MONTGOMERY)
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

void Fnec_eq(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->shortVal = Fnec_req(r, a, b);
    r->type = Fnec_SHORT;
}

void Fnec_neq(PFnecElement r, PFnecElement a, PFnecElement b)
{
    int result = Fnec_req(r, a, b);

    r->shortVal = result ^ 0x1;
    r->type = Fnec_SHORT;
}

// Logical or between two elements
void Fnec_lor(PFnecElement r, PFnecElement a, PFnecElement b)
{
    int32_t is_true_a;

    if (a->type & Fnec_LONG)
    {
        is_true_a = !Fnec_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & Fnec_LONG)
    {
        is_true_b = !Fnec_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a | is_true_b;
    r->type = Fnec_SHORT;
}

void Fnec_lnot(PFnecElement r, PFnecElement a)
{
    if (a->type & Fnec_LONG)
    {
        r->shortVal = Fnec_rawIsZero(a->longVal);
    }
    else
    {
        r->shortVal = a->shortVal ? 0 : 1;
    }

    r->type = Fnec_SHORT;
}


static inline int rgt_s1s2(PFnecElement a, PFnecElement b)
{
    return (a->shortVal > b->shortVal) ? 1 : 0;
}

static inline int rgtRawL1L2(FnecRawElement pRawA, FnecRawElement pRawB)
{
    int result = Fnec_rawCmp(pRawB, pRawA);

    return (result < 0) ? 1 : 0;
}

static inline int rgtl1l2_n1(FnecRawElement pRawA, FnecRawElement pRawB)
{
    int result = Fnec_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rgtRawL1L2(pRawA, pRawB);
    }
    return 0;
}

static inline int rgtl1l2_p1(FnecRawElement pRawA, FnecRawElement pRawB)
{
    int result = Fnec_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 1;
    }
    return rgtRawL1L2(pRawA, pRawB);
}

static inline int rgtL1L2(FnecRawElement pRawA, FnecRawElement pRawB)
{
    int result = Fnec_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rgtl1l2_n1(pRawA, pRawB);
    }

    return rgtl1l2_p1(pRawA, pRawB);
}

static inline int rgt_l1nl2n(PFnecElement a, PFnecElement b)
{
    return rgtL1L2(a->longVal, b->longVal);
}

static inline int rgt_l1nl2m(PFnecElement a, PFnecElement b)
{
    FnecElement b_n;
    Fnec_toNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

static inline int rgt_l1ml2m(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);
    Fnec_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ml2n(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    Fnec_toNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_s1l2n(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    Fnec_toLongNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_l1ms2(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);
    Fnec_toLongNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_s1l2m(PFnecElement a, PFnecElement b)
{
    FnecElement a_n;
    FnecElement b_n;

    Fnec_toLongNormal(&a_n, a);
    Fnec_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ns2(PFnecElement a, PFnecElement b)
{
    FnecElement b_n;
    Fnec_toLongNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

int Fnec_rgt(PFnecElement r, PFnecElement a, PFnecElement b)
{
    int result = 0;

    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
                {
                    result = rgt_l1ml2m(a, b);
                }
                else
                {
                    result = rgt_l1ml2n(a, b);
                }
            }
            else if (b->type & Fnec_MONTGOMERY)
            {
                result = rgt_l1nl2m(a, b);
            }
            else
            {
                result = rgt_l1nl2n(a, b);
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            result = rgt_l1ms2(a, b);
        }
        else
        {
            result = rgt_l1ns2(a, b);
        }
    }
    else if (b->type & Fnec_LONG)
    {
        if (b->type & Fnec_MONTGOMERY)
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

void Fnec_gt(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->shortVal = Fnec_rgt(r, a, b);
    r->type = Fnec_SHORT;
}

void Fnec_leq(PFnecElement r, PFnecElement a, PFnecElement b)
{
   int32_t result = Fnec_rgt(r, a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = Fnec_SHORT;
}

// Logical and between two elements
void Fnec_land(PFnecElement r, PFnecElement a, PFnecElement b)
{
    int32_t is_true_a;

    if (a->type & Fnec_LONG)
    {
        is_true_a = !Fnec_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & Fnec_LONG)
    {
        is_true_b = !Fnec_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a & is_true_b;
    r->type = Fnec_SHORT;
}

static inline void and_s1s2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        int32_t result = a->shortVal & b->shortVal;
        r->shortVal = result;
        r->type = Fnec_SHORT;
        return;
    }

    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toLongNormal(&a_n, a);
    Fnec_toLongNormal(&b_n, b);

    Fnec_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1nl2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;
    Fnec_rawAnd(r->longVal, a->longVal, b->longVal);
}

static inline void and_l1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement b_n;
    Fnec_toNormal(&b_n, b);

    Fnec_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

static inline void and_l1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);
    Fnec_toNormal(&b_n, b);

    Fnec_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ml2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    Fnec_toNormal(&a_n, a);

    Fnec_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_s1l2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&a_n, a);
    }

    Fnec_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_l1ms2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&b_n, b);
    }

    Fnec_rawAnd(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void and_s1l2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&a_n, a);
    }

    Fnec_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ns2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&b_n, b);
    }

    Fnec_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

// Ands two elements of any kind
void Fnec_band(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
                {
                    and_l1ml2m(r, a, b);
                }
                else
                {
                    and_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fnec_MONTGOMERY)
            {
                and_l1nl2m(r, a, b);
            }
            else
            {
                and_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            and_l1ms2(r, a, b);
        }
        else
        {
           and_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fnec_LONG)
    {
        if (b->type & Fnec_MONTGOMERY)
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

void Fnec_rawZero(FnecRawElement pRawResult)
{
    std::memset(pRawResult, 0, sizeof(FnecRawElement));
}

static inline void rawShl(FnecRawElement r, FnecRawElement a, uint64_t b)
{
    if (b == 0)
    {
        Fnec_rawCopy(r, a);
        return;
    }

    if (b >= 256)
    {
        Fnec_rawZero(r);
        return;
    }

    Fnec_rawShl(r, a, b);
}

static inline void rawShr(FnecRawElement r, FnecRawElement a, uint64_t b)
{
    if (b == 0)
    {
        Fnec_rawCopy(r, a);
        return;
    }

    if (b >= 256)
    {
        Fnec_rawZero(r);
        return;
    }

    Fnec_rawShr(r,a, b);
}

static inline void Fnec_setzero(PFnecElement r)
{
    r->type = 0;
    r->shortVal = 0;
}

static inline void do_shlcl(PFnecElement r, PFnecElement a, uint64_t b)
{
    FnecElement a_long;
    Fnec_toLongNormal(&a_long, a);

    r->type = Fnec_LONG;
    rawShl(r->longVal, a_long.longVal, b);
}

static inline void do_shlln(PFnecElement r, PFnecElement a, uint64_t b)
{
    r->type = Fnec_LONG;
    rawShl(r->longVal, a->longVal, b);
}

static inline void do_shl(PFnecElement r, PFnecElement a, uint64_t b)
{
    if (a->type & Fnec_LONG)
    {
        if (a->type == Fnec_LONGMONTGOMERY)
        {
            FnecElement a_long;
            Fnec_toNormal(&a_long, a);

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
            Fnec_setzero(r);
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
                r->type = Fnec_SHORT;
                r->shortVal = a_shortVal;
            }
        }
    }
}

static inline void do_shrln(PFnecElement r, PFnecElement a, uint64_t b)
{
    r->type = Fnec_LONG;
    rawShr(r->longVal, a->longVal, b);
}

static inline void do_shrl(PFnecElement r, PFnecElement a, uint64_t b)
{
    if (a->type == Fnec_LONGMONTGOMERY)
    {
        FnecElement a_long;
        Fnec_toNormal(&a_long, a);

        do_shrln(r, &a_long, b);
    }
    else
    {
        do_shrln(r, a, b);
    }
}

static inline void do_shr(PFnecElement r, PFnecElement a, uint64_t b)
{
    if (a->type & Fnec_LONG)
    {
        do_shrl(r, a, b);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            Fnec_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            FnecElement a_long;
            Fnec_toLongNormal(&a_long, a);

            do_shrl(r, &a_long, b);
        }
        else if(b >= 31)
        {
            Fnec_setzero(r);
        }
        else
        {
            a_shortVal >>= b;

            r->shortVal = a_shortVal;
            r->type = Fnec_SHORT;
        }
    }
}

static inline void Fnec_shr_big_shift(PFnecElement r, PFnecElement a, PFnecElement b)
{
    static FnecRawElement max_shift = {256};

    FnecRawElement shift;

    Fnec_rawSubRegular(shift, Fnec_q.longVal, b->longVal);

    if (Fnec_rawCmp(shift, max_shift) >= 0)
    {
        Fnec_setzero(r);
    }
    else
    {
        do_shl(r, a, shift[0]);
    }
}

static inline void Fnec_shr_long(PFnecElement r, PFnecElement a, PFnecElement b)
{
    static FnecRawElement max_shift = {256};

    if (Fnec_rawCmp(b->longVal, max_shift) >= 0)
    {
        Fnec_shr_big_shift(r, a, b);
    }
    else
    {
        do_shr(r, a, b->longVal[0]);
    }
}

void Fnec_shr(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (b->type & Fnec_LONG)
    {
        if (b->type == Fnec_LONGMONTGOMERY)
        {
            FnecElement b_long;
            Fnec_toNormal(&b_long, b);

            Fnec_shr_long(r, a, &b_long);
        }
        else
        {
            Fnec_shr_long(r, a, b);
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
                Fnec_setzero(r);
            }
            else
            {
                do_shl(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 256)
        {
            Fnec_setzero(r);
        }
        else
        {
            do_shr(r, a, b_shortVal);
        }
    }
}

static inline void Fnec_shl_big_shift(PFnecElement r, PFnecElement a, PFnecElement b)
{
    static FnecRawElement max_shift = {256};

    FnecRawElement shift;

    Fnec_rawSubRegular(shift, Fnec_q.longVal, b->longVal);

    if (Fnec_rawCmp(shift, max_shift) >= 0)
    {
        Fnec_setzero(r);
    }
    else
    {
        do_shr(r, a, shift[0]);
    }
}

static inline void Fnec_shl_long(PFnecElement r, PFnecElement a, PFnecElement b)
{
    static FnecRawElement max_shift = {256};

    if (Fnec_rawCmp(b->longVal, max_shift) >= 0)
    {
        Fnec_shl_big_shift(r, a, b);
    }
    else
    {
        do_shl(r, a, b->longVal[0]);
    }
}

void Fnec_shl(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (b->type & Fnec_LONG)
    {
        if (b->type == Fnec_LONGMONTGOMERY)
        {
            FnecElement b_long;
            Fnec_toNormal(&b_long, b);

            Fnec_shl_long(r, a, &b_long);
        }
        else
        {
            Fnec_shl_long(r, a, b);
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
                Fnec_setzero(r);
            }
            else
            {
                do_shr(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 256)
        {
            Fnec_setzero(r);
        }
        else
        {
            do_shl(r, a, b_shortVal);
        }
    }
}

void Fnec_square(PFnecElement r, PFnecElement a)
{
    if (a->type & Fnec_LONG)
    {
        if (a->type == Fnec_LONGMONTGOMERY)
        {
            r->type = Fnec_LONGMONTGOMERY;
            Fnec_rawMSquare(r->longVal, a->longVal);
        }
        else
        {
            r->type = Fnec_LONGMONTGOMERY;
            Fnec_rawMSquare(r->longVal, a->longVal);
            Fnec_rawMMul(r->longVal, r->longVal, Fnec_R3.longVal);
        }
    }
    else
    {
        int64_t result;

        int overflow = Fnec_rawSMul(&result, a->shortVal, a->shortVal);

        if (overflow)
        {
            Fnec_rawCopyS2L(r->longVal, result);
            r->type = Fnec_LONG;
            r->shortVal = 0;
        }
        else
        {
            // done the same way as in intel asm implementation
            r->shortVal = (int32_t)result;
            r->type = Fnec_SHORT;
            //

            Fnec_rawCopyS2L(r->longVal, result);
            r->type = Fnec_LONG;
            r->shortVal = 0;
        }
    }
}

static inline void or_s1s2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal | b->shortVal;
        r->type = Fnec_SHORT;
        return;
    }

    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toLongNormal(&a_n, a);
    Fnec_toLongNormal(&b_n, b);

    Fnec_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&a_n, a);
    }

    Fnec_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&a_n, a);
    }

    Fnec_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ns2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&b_n, b);
    }

    Fnec_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ms2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&b_n, b);
    }

    Fnec_rawOr(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void or_l1nl2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;
    Fnec_rawOr(r->longVal, a->longVal, b->longVal);
}

static inline void or_l1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement b_n;
    Fnec_toNormal(&b_n, b);

    Fnec_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ml2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    Fnec_toNormal(&a_n, a);

    Fnec_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);
    Fnec_toNormal(&b_n, b);

    Fnec_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}


void Fnec_bor(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
                {
                    or_l1ml2m(r, a, b);
                }
                else
                {
                    or_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fnec_MONTGOMERY)
            {
                or_l1nl2m(r, a, b);
            }
            else
            {
                or_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            or_l1ms2(r, a, b);
        }
        else
        {
           or_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fnec_LONG)
    {
        if (b->type & Fnec_MONTGOMERY)
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

static inline void xor_s1s2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal ^ b->shortVal;
        r->type = Fnec_SHORT;
        return;
    }

    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toLongNormal(&a_n, a);
    Fnec_toLongNormal(&b_n, b);

    Fnec_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_s1l2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&a_n, a);
    }

    Fnec_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_s1l2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&a_n, a);
    }

    Fnec_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_l1ns2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&b_n, b);
    }

    Fnec_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ms2(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal}};
    }
    else
    {
        Fnec_toLongNormal(&b_n, b);
    }

    Fnec_rawXor(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void xor_l1nl2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;
    Fnec_rawXor(r->longVal, a->longVal, b->longVal);
}

static inline void xor_l1nl2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement b_n;
    Fnec_toNormal(&b_n, b);

    Fnec_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ml2n(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    Fnec_toNormal(&a_n, a);

    Fnec_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_l1ml2m(PFnecElement r, PFnecElement a, PFnecElement b)
{
    r->type = Fnec_LONG;

    FnecElement a_n;
    FnecElement b_n;

    Fnec_toNormal(&a_n, a);
    Fnec_toNormal(&b_n, b);

    Fnec_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

void Fnec_bxor(PFnecElement r, PFnecElement a, PFnecElement b)
{
    if (a->type & Fnec_LONG)
    {
        if (b->type & Fnec_LONG)
        {
            if (a->type & Fnec_MONTGOMERY)
            {
                if (b->type & Fnec_MONTGOMERY)
                {
                    xor_l1ml2m(r, a, b);
                }
                else
                {
                    xor_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fnec_MONTGOMERY)
            {
                xor_l1nl2m(r, a, b);
            }
            else
            {
                xor_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fnec_MONTGOMERY)
        {
            xor_l1ms2(r, a, b);
        }
        else
        {
           xor_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fnec_LONG)
    {
        if (b->type & Fnec_MONTGOMERY)
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

void Fnec_bnot(PFnecElement r, PFnecElement a)
{
    r->type = Fnec_LONG;

    if (a->type == Fnec_LONG)
    {
        if (a->type & Fnec_MONTGOMERY)
        {
            FnecElement a_n;
            Fnec_toNormal(&a_n, a);

            Fnec_rawNot(r->longVal, a_n.longVal);
        }
        else
        {
            Fnec_rawNot(r->longVal, a->longVal);
        }
    }
    else
    {
        FnecElement a_n;
        Fnec_toLongNormal(&a_n, a);

        Fnec_rawNot(r->longVal, a_n.longVal);
    }
}
