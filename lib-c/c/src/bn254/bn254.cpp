
#include <gmpxx.h>
#include "bn254.hpp"
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

int inline BN254CurveAddFe (const RawFq::Element &x1, const RawFq::Element &y1, const RawFq::Element &x2, const RawFq::Element &y2, RawFq::Element &x3, RawFq::Element &y3)
{
    RawFq::Element aux1, aux2, s;

    // s = (y2-y1)/(x2-x1)
    bn254.sub(aux1, y2, y1);
    bn254.sub(aux2, x2, x1);
    if (bn254.isZero(aux2))
    {
        printf("BN254CurveAddFe() got denominator=0 2\n");
        return -1;
    }
    bn254.div(s, aux1, aux2);

    // Required for x3 calculation
    bn254.add(aux2, x1, x2);

    // x3 = s*s - (x1+x2)
    bn254.mul(aux1, s, s);
    // aux2 was calculated before
    bn254.sub(x3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    bn254.sub(aux1, x1, x3);;
    bn254.mul(aux1, aux1, s);
    bn254.sub(y3, aux1, y1);

    return 0;
}

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

int inline BN254CurveDblFe (const RawFq::Element &x1, const RawFq::Element &y1, RawFq::Element &x2, RawFq::Element &y2)
{
    RawFq::Element aux1, aux2, s;

    // s = 3*x1*x1/2*y1
    bn254.mul(aux1, x1, x1);
    bn254.fromUI(aux2, 3);
    bn254.mul(aux1, aux1, aux2);
    bn254.add(aux2, y1, y1);
    if (bn254.isZero(aux2))
    {
        printf("BN254CurveDblFe() got denominator=0 1\n");
        return -1;
    }
    bn254.div(s, aux1, aux2);

    // Required for x3 calculation
    bn254.add(aux2, x1, x1);

    // x2 = s*s - (x1+x2)
    bn254.mul(aux1, s, s);
    // aux2 was calculated before
    bn254.sub(x2, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    bn254.sub(aux1, x1, x2);;
    bn254.mul(aux1, aux1, s);
    bn254.sub(y2, aux1, y1);

    return 0;
}

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

int inline BN254ComplexAddFe (const RawFq::Element &x1, const RawFq::Element &y1, const RawFq::Element &x2, const RawFq::Element &y2, RawFq::Element &x3, RawFq::Element &y3)
{
    // Addition of 2 complex numbers:
    // x3 = x1 + x2 -> real parts are added
    // y3 = y1 + y2 -> imaginary parts are added

    bn254.add(x3, x1, x2);
    bn254.add(y3, y1, y2);

    return 0;
}

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

int inline BN254ComplexSubFe (const RawFq::Element &x1, const RawFq::Element &y1, const RawFq::Element &x2, const RawFq::Element &y2, RawFq::Element &x3, RawFq::Element &y3)
{
    // Subtraction of 2 complex numbers:
    // x3 = x1 - x2 -> real parts are subtracted
    // y3 = y1 - y2 -> imaginary parts are subtracted
    
    bn254.sub(x3, x1, x2);
    bn254.sub(y3, y1, y2);

    return 0;
}

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

int inline BN254ComplexMulFe (const RawFq::Element &x1, const RawFq::Element &y1, const RawFq::Element &x2, const RawFq::Element &y2, RawFq::Element &x3, RawFq::Element &y3)
{
    // Multiplication of 2 complex numbers:
    // x3 = x1 * x2 - y1 * y2 -> real parts are multiplied, minus the multiplication of the imaginary parts (i*i = -1)
    // y3 = y1 * x2 + x1 * y2 -> imaginary parts are multiplied by the opposite real parts

    RawFq::Element aux1, aux2;

    bn254.mul(aux1, x1, x2);
    bn254.mul(aux2, y1, y2);
    bn254.sub(x3, aux1, aux2);
    
    bn254.mul(aux1, y1, x2);
    bn254.mul(aux2, x1, y2);
    bn254.add(y3, aux1, aux2);

    return 0;
}

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