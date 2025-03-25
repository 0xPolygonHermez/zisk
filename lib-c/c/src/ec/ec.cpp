
#include <gmpxx.h>
#include "ec.hpp"
#include "../ffiasm/fec.hpp"
#include "../ffiasm/fnec.hpp"

RawFec fec;
RawFnec fnec;

inline void array2scalar (const uint64_t * a, mpz_class &s)
{
    mpz_import(s.get_mpz_t(), 4, -1, 8, -1, 0, (const void *)a);
}

inline void scalar2array (mpz_class &s, uint64_t * a)
{
    mpz_export((void *)a, NULL, -1, 8, -1, 0, s.get_mpz_t());
}

inline void array2fe (const uint64_t * a, RawFec::Element &fe)
{
    mpz_class s;
    array2scalar(a, s);
    fec.fromMpz(fe, s.get_mpz_t());
}

inline void fe2array (const RawFec::Element &fe, uint64_t * a)
{
    mpz_class s;
    fec.toMpz(s.get_mpz_t(), fe);
    scalar2array(s, a);
}

inline void array2fe (const uint64_t * a, RawFnec::Element &fe)
{
    mpz_class s;
    array2scalar(a, s);
    fnec.fromMpz(fe, s.get_mpz_t());
}

inline void fe2array (const RawFnec::Element &fe, uint64_t * a)
{
    mpz_class s;
    fnec.toMpz(s.get_mpz_t(), fe);
    scalar2array(s, a);
}

#ifdef __cplusplus
extern "C" {
#endif

int inline AddPointEcFe (bool dbl, const RawFec::Element &x1, const RawFec::Element &y1, const RawFec::Element &x2, const RawFec::Element &y2, RawFec::Element &x3, RawFec::Element &y3)
{
    // Check if results are buffered
#ifdef ENABLE_EXPERIMENTAL_CODE
    if(ctx.ecRecoverPrecalcBuffer.filled == true){
        if(ctx.ecRecoverPrecalcBuffer.pos < 2){
            zklog.error("ecRecoverPrecalcBuffer.buffer buffer is not filled, but pos < 2 (pos=" + to_string(ctx.ecRecoverPrecalcBuffer.pos) + ")");
            exitProcess();
        }
        x3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-2];
        y3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-1];
        return ZKR_SUCCESS;
    }
#endif

    RawFec::Element aux1, aux2, s;

    if (dbl)
    {
        // s = 3*x1*x1/2*y1
        fec.mul(aux1, x1, x1);
        fec.fromUI(aux2, 3);
        fec.mul(aux1, aux1, aux2);
        fec.add(aux2, y1, y1);
        if (fec.isZero(aux2))
        {
            printf("AddPointEc() got denominator=0 1\n");
            return -1;
        }
        fec.div(s, aux1, aux2);
    }
    else
    {
        // s = (y2-y1)/(x2-x1)
        fec.sub(aux1, y2, y1);
        fec.sub(aux2, x2, x1);
        if (fec.isZero(aux2))
        {
            printf("AddPointEc() got denominator=0 2\n");
            return -1;
        }
        fec.div(s, aux1, aux2);
    }

    // x3 = s*s - (x1+x2)
    fec.mul(aux1, s, s);
    fec.add(aux2, x1, x2);
    fec.sub(x3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    fec.sub(aux1, x1, x3);;
    fec.mul(aux1, aux1, s);
    fec.sub(y3, aux1, y1);

    return 0;
}

int AddPointEc (uint64_t _dbl, const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    bool dbl = _dbl;

    RawFec::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    if (!dbl)
    {
        array2fe(_x2, x2);
        array2fe(_y2, y2);
    }

    int result = AddPointEcFe (dbl, x1, y1, x2, y2, x3, y3);
    
    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int AddPointEcP (uint64_t _dbl, const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    bool dbl = _dbl;

    RawFec::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    if (!dbl)
    {
        array2fe(p2, x2);
        array2fe(p2 + 4, y2);
    }

    // printf("AddPointEcP() x1=%s\n", fec.toString(x1, 16).c_str());
    // printf("AddPointEcP() y1=%s\n", fec.toString(y1, 16).c_str());
    // printf("AddPointEcP() x2=%s\n", fec.toString(x2, 16).c_str());
    // printf("AddPointEcP() y2=%s\n", fec.toString(y2, 16).c_str());

    int result = AddPointEcFe (dbl, x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

int InverseFpEc (
    const unsigned long * _a,  // 8 x 64 bits
    unsigned long * _r  // 8 x 64 bits
)
{
    // TODO: call mpz_invert
    RawFec::Element a;
    array2fe(_a, a);
    if (fec.isZero(a))
    {
        printf("InverseFpEc() Division by zero\n");
        return -1;
    }

    RawFec::Element r;
    fec.inv(r, a);

    fe2array(r, _r);

    return 0;
}

int InverseFnEc (
    const unsigned long * _a,  // 8 x 64 bits
    unsigned long * _r  // 8 x 64 bits
)
{
    RawFnec::Element a;
    array2fe(_a, a);
    if (fnec.isZero(a))
    {
        printf("InverseFnEc() Division by zero\n");
        return -1;
    }

    RawFnec::Element r;
    fnec.inv(r, a);

    fe2array(r, _r);

    return 0;
}

mpz_class n("0x3fffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffff0c");
mpz_class p("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F");
mpz_class ScalarMask256 ("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16);

// We use that p = 3 mod 4 => r = a^((p+1)/4) is a square root of a
// https://www.rieselprime.de/ziki/Modular_square_root
// n = p+1/4

inline void sqrtF3mod4(mpz_class &r, const mpz_class &a)
{
    mpz_class auxa = a;
    mpz_powm(r.get_mpz_t(), a.get_mpz_t(), n.get_mpz_t(), p.get_mpz_t());
    if ((r * r) % p != auxa)
    {
        r = ScalarMask256;
    }
}

int SqrtFpEcParity (
    const unsigned long * _a,  // 8 x 64 bits
    const unsigned long _parity,  // 8 x 64 bits
    unsigned long * _r  // 8 x 64 bits
)
{
    mpz_class parity = _parity;
    mpz_class a;
    array2scalar(_a, a);

    // Call the sqrt function
    mpz_class r;
    sqrtF3mod4(r, a);

    // Post-process the result
    if (r == ScalarMask256)
    {
        // This sqrt does not have a solution
    }
    else if ((r & 1) == parity)
    {
        // Return r as it is, since it has the requested parity
    }
    else
    {
        // Negate the result
        RawFec::Element fe;
        fec.fromMpz(fe, r.get_mpz_t());
        fe = fec.neg(fe);
        fec.toMpz(r.get_mpz_t(), fe);
    }
    return 0;
}

int Arith256 (
    const unsigned long * _a,  // 4 x 64 bits
    const unsigned long * _b,  // 4 x 64 bits
    const unsigned long * _c,  // 4 x 64 bits
    unsigned long * _dl, // 4 x 64 bits
    unsigned long * _dh // 4 x 64 bits
)
{
    // Convert input parameters to scalars
    mpz_class a, b, c;
    array2scalar(_a, a);
    array2scalar(_b, b);
    array2scalar(_c, c);

    // Calculate the result as a scalar
    mpz_class d;
    d = (a * b) + c;

    // Decompose d = dl + dh<<256 (dh = d)
    mpz_class dl;
    dl = d & ScalarMask256;
    d >>= 256;

    // Convert scalars to output parameters
    scalar2array(dl, _dl);
    scalar2array(d, _dh);

    return 0;
}

int Arith256Mod (
    const unsigned long * _a,  // 4 x 64 bits
    const unsigned long * _b,  // 4 x 64 bits
    const unsigned long * _c,  // 4 x 64 bits
    const unsigned long * _module,  // 4 x 64 bits
    unsigned long * _d // 4 x 64 bits
)
{
    // Convert input parameters to scalars
    mpz_class a, b, c, module;
    array2scalar(_a, a);
    array2scalar(_b, b);
    array2scalar(_c, c);
    array2scalar(_module, module);

    // Calculate the result as a scalar
    mpz_class d;
    d = ((a * b) + c) % module;

    // Convert scalar to output parameter
    scalar2array(d, _d);

    return 0;
}

#ifdef __cplusplus
} // extern "C"
#endif