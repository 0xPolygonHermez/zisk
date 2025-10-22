
#include <gmpxx.h>
#include "ec.hpp"
#include "../ffiasm/fec.hpp"
#include "../ffiasm/fnec.hpp"
#include "../common/utils.hpp"
#include "../common/globals.hpp"
#include <stdint.h>

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

        // Required for x3 calculation
        fec.add(aux2, x1, x1);
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

        // Required for x3 calculation
        fec.add(aux2, x1, x2);
    }

    // x3 = s*s - (x1+x2)
    fec.mul(aux1, s, s);
    // aux2 was calculated before
    fec.sub(x3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    fec.sub(aux1, x1, x3);;
    fec.mul(aux1, aux1, s);
    fec.sub(y3, aux1, y1);

    return 0;
}

int inline AddPointEcDblFe (RawFec::Element &x1, RawFec::Element &y1)
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

    RawFec::Element aux1, aux2, aux3, s;

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

    // Required for x3 calculation
    fec.add(aux2, x1, x1);

    // x3 = s*s - (x1+x2)
    fec.mul(aux1, s, s);
    // aux2 was calculated before
    
    fec.sub(aux3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    fec.sub(aux1, x1, aux3);
    x1 = aux3;
    fec.mul(aux1, aux1, s);
    fec.sub(y1, aux1, y1);

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

int AddPointEcDbl (uint64_t * _x1, uint64_t * _y1)
{
    RawFec::Element x1, y1;
    array2fe(_x1, x1);
    array2fe(_y1, y1);

    int result = AddPointEcDblFe (x1, y1);
    
    fe2array(x1, _x1);
    fe2array(y1, _y1);

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

#ifdef __cplusplus
} // extern "C"
#endif