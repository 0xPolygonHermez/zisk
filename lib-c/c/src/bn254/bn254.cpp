
#include <gmpxx.h>
#include "bn254.hpp"
#include "bn254_fe.hpp"
#include "../ffiasm/fq.hpp"
#include "../common/utils.hpp"
#include "../common/globals.hpp"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*******************/
/* BN254 CURVE ADD */
/*******************/

int BN254CurveAdd (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawFq::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = BN254CurveAddFe (x1, y1, x2, y2, x3, y3);
    
    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int BN254CurveAddP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawFq::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    int result = BN254CurveAddFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

/**********************/
/* BN254 CURVE DOUBLE */
/**********************/

int BN254CurveDbl (const uint64_t * _x1, const uint64_t * _y1, uint64_t * _x2, uint64_t * _y2)
{
    RawFq::Element x1, y1, x2, y2;
    array2fe(_x1, x1);
    array2fe(_y1, y1);

    int result = BN254CurveDblFe (x1, y1, x2, y2);
    
    fe2array(x2, _x2);
    fe2array(y2, _y2);

    return result;
}

int BN254CurveDblP (const uint64_t * p1, uint64_t * p2)
{
    RawFq::Element x1, y1, x2, y2;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);

    int result = BN254CurveDblFe (x1, y1, x2, y2);

    fe2array(x2, p2);
    fe2array(y2, p2 + 4);

    return result;
}

/*********************/
/* BN254 COMPLEX ADD */
/*********************/

int BN254ComplexAdd (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawFq::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = BN254ComplexAddFe (x1, y1, x2, y2, x3, y3);
    
    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int BN254ComplexAddP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawFq::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    int result = BN254ComplexAddFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

/*********************/
/* BN254 COMPLEX SUB */
/*********************/

int BN254ComplexSub (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawFq::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = BN254ComplexSubFe (x1, y1, x2, y2, x3, y3);
    
    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int BN254ComplexSubP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawFq::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    int result = BN254ComplexSubFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

/*********************/
/* BN254 COMPLEX MUL */
/*********************/

int BN254ComplexMul (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawFq::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    int result = BN254ComplexMulFe (x1, y1, x2, y2, x3, y3);
    
    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int BN254ComplexMulP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawFq::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    array2fe(p2, x2);
    array2fe(p2 + 4, y2);

    int result = BN254ComplexMulFe (x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

#ifdef __cplusplus
} // extern "C"
#endif