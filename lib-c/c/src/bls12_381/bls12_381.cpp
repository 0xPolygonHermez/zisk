
#include <gmpxx.h>
#include "bls12_381.hpp"
#include "bls12_381_fe.hpp"
#include "../ffiasm/bls12_381_384.hpp"
#include "../common/utils.hpp"
#include "../common/globals.hpp"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/***********************/
/* BLS12_381 CURVE ADD */
/***********************/

int BLS12_381CurveAdd (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = BLS12_381CurveAddFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int BLS12_381CurveAddP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    int result = BLS12_381CurveAddFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

/**************************/
/* BLS12_381 CURVE DOUBLE */
/**************************/

int BLS12_381CurveDbl (const uint64_t * _x1, const uint64_t * _y1, uint64_t * _x2, uint64_t * _y2)
{
    RawBLS12_381_384::Element x1, y1, x2, y2;
    array2fe(_x1, x1);
    array2fe(_y1, y1);

    int result = BLS12_381CurveDblFe (x1, y1, x2, y2);

    fe2array(x2, _x2);
    fe2array(y2, _y2);

    return result;
}

int BLS12_381CurveDblP (const uint64_t * p1, uint64_t * p2)
{
    RawBLS12_381_384::Element x1, y1, x2, y2;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);

    int result = BLS12_381CurveDblFe (x1, y1, x2, y2);

    fe2array(x2, p2);
    fe2array(y2, p2 + 4);

    return result;
}

/*************************/
/* BLS12_381 COMPLEX ADD */
/*************************/

int BLS12_381ComplexAdd (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = BLS12_381ComplexAddFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int BLS12_381ComplexAddP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    int result = BLS12_381ComplexAddFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

/*************************/
/* BLS12_381 COMPLEX SUB */
/*************************/

int BLS12_381ComplexSub (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = BLS12_381ComplexSubFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int BLS12_381ComplexSubP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    int result = BLS12_381ComplexSubFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

/*************************/
/* BLS12_381 COMPLEX MUL */
/*************************/

int BLS12_381ComplexMul (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = BLS12_381ComplexMulFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int BLS12_381ComplexMulP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    int result = BLS12_381ComplexMulFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

#ifdef __cplusplus
} // extern "C"
#endif