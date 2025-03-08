#include <gmpxx.h>
#include "ec.hpp"
#include "../ffiasm/fec.hpp"

RawFec fec;

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

    RawFec::Element x1;
    x1.v[0] = _x1[0];
    x1.v[1] = _x1[1];
    x1.v[2] = _x1[2];
    x1.v[3] = _x1[3];

    RawFec::Element y1;
    y1.v[0] = _y1[0];
    y1.v[1] = _y1[1];
    y1.v[2] = _y1[2];
    y1.v[3] = _y1[3];

    RawFec::Element x2;
    x2.v[0] = _x2[0];
    x2.v[1] = _x2[1];
    x2.v[2] = _x2[2];
    x2.v[3] = _x2[3];

    RawFec::Element y2;
    y2.v[0] = _y2[0];
    y2.v[1] = _y2[1];
    y2.v[2] = _y2[2];
    y2.v[3] = _y2[3];

    RawFec::Element x3;

    RawFec::Element y3;

    int result = AddPointEc (dbl, x1, y1, x2, y2, x3, y3);

    _x3[0] = x3.v[0];
    _x3[1] = x3.v[1];
    _x3[2] = x3.v[2];
    _x3[3] = x3.v[3];

    _y3[0] = y3.v[0];
    _y3[1] = y3.v[1];
    _y3[2] = y3.v[2];
    _y3[3] = y3.v[3];

    return result;
}