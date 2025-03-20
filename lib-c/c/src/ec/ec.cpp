#include <gmpxx.h>
#include "ec.hpp"
#include "../ffiasm/fec.hpp"

RawFec fec;

inline void array2scalar (const uint64_t * a, mpz_class &s)
{
    s = a[3];
    s <<= 64;
    s += a[2];
    s <<= 64;
    s += a[1];
    s <<= 64;
    s += a[0];
}

inline void array2fe (const uint64_t * a, RawFec::Element &fe)
{
    mpz_class s;
    array2scalar(a, s);
    fec.fromMpz(fe, s.get_mpz_t());
}

inline void scalar2array (mpz_class &s, uint64_t * a)
{
    a[0] = s.get_ui();
    s >>= 64;
    a[1] = s.get_ui();
    s >>= 64;
    a[2] = s.get_ui();
    s >>= 64;
    a[3] = s.get_ui();
}

inline void fe2array (const RawFec::Element &fe, uint64_t * a)
{
    mpz_class s;
    fec.toMpz(s.get_mpz_t(), fe);
    scalar2array(s, a);
}

int inline AddPointEc (bool dbl, const RawFec::Element &x1, const RawFec::Element &y1, const RawFec::Element &x2, const RawFec::Element &y2, RawFec::Element &x3, RawFec::Element &y3)
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
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = AddPointEc (dbl, x1, y1, x2, y2, x3, y3);
    
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
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    // printf("AddPointEcP() x1=%s\n", fec.toString(x1, 16).c_str());
    // printf("AddPointEcP() y1=%s\n", fec.toString(y1, 16).c_str());
    // printf("AddPointEcP() x2=%s\n", fec.toString(x2, 16).c_str());
    // printf("AddPointEcP() y2=%s\n", fec.toString(y2, 16).c_str());

    int result = AddPointEc (dbl, x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}