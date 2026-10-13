
#include <gmpxx.h>
#include "bls12_381.hpp"
#include "bls12_381_fe.hpp"
#include "../ffiasm/bls12_381_384.hpp"
#include "../common/utils.hpp"
#include "../common/curve_plain.hpp"
#include "../common/complex_plain.hpp"
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
    // Works on plain values, see curve_plain.hpp
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2plain(p1, x1);
    array2plain(p1 + 6, y1);
    array2plain(p2, x2);
    array2plain(p2 + 6, y2);

    int result = curve_add_plain(bls12_381, "BLS12_381CurveAddFe()", x1, y1, x2, y2, x3, y3);

    plain2array(x3, p3);
    plain2array(y3, p3 + 6);

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
    // Works on plain values, see curve_plain.hpp
    RawBLS12_381_384::Element x1, y1, x2, y2;
    array2plain(p1, x1);
    array2plain(p1 + 6, y1);

    int result = curve_dbl_plain(bls12_381, "BLS12_381CurveDblFe()", false, x1, y1, x2, y2);

    plain2array(x2, p2);
    plain2array(y2, p2 + 6);

    return result;
}

/*************************/
/* BLS12_381 COMPLEX ADD */
/*************************/

// Works on plain values: a complex add gives the same result as on Montgomery ones
int BLS12_381ComplexAdd (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2plain(_x1, x1);
    array2plain(_y1, y1);
    array2plain(_x2, x2);
    array2plain(_y2, y2);

    int result = BLS12_381ComplexAddFe (x1, y1, x2, y2, x3, y3);

    plain2array(x3, _x3);
    plain2array(y3, _y3);

    return result;
}

// Works on plain values: a complex add gives the same result as on Montgomery ones
int BLS12_381ComplexAddP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2plain(p1, x1);
    array2plain(p1 + 6, y1);
    array2plain(p2, x2);
    array2plain(p2 + 6, y2);

    int result = BLS12_381ComplexAddFe (x1, y1, x2, y2, x3, y3);

    plain2array(x3, p3);
    plain2array(y3, p3 + 6);

    return result;
}

/*************************/
/* BLS12_381 COMPLEX SUB */
/*************************/

// Works on plain values: a complex sub gives the same result as on Montgomery ones
int BLS12_381ComplexSub (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2plain(_x1, x1);
    array2plain(_y1, y1);
    array2plain(_x2, x2);
    array2plain(_y2, y2);

    int result = BLS12_381ComplexSubFe (x1, y1, x2, y2, x3, y3);

    plain2array(x3, _x3);
    plain2array(y3, _y3);

    return result;
}

// Works on plain values: a complex sub gives the same result as on Montgomery ones
int BLS12_381ComplexSubP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2plain(p1, x1);
    array2plain(p1 + 6, y1);
    array2plain(p2, x2);
    array2plain(p2 + 6, y2);

    int result = BLS12_381ComplexSubFe (x1, y1, x2, y2, x3, y3);

    plain2array(x3, p3);
    plain2array(y3, p3 + 6);

    return result;
}

/*************************/
/* BLS12_381 COMPLEX MUL */
/*************************/

// Only the second operand is converted to Montgomery form, and the product uses Karatsuba,
// see complex_plain.hpp
int BLS12_381ComplexMul (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2plain(_x1, x1);
    array2plain(_y1, y1);
    array2fe(_x2, x2);
    array2fe(_y2, y2);

    complex_mul_plain(bls12_381, x1, y1, x2, y2, x3, y3);
    int result = 0;

    plain2array(x3, _x3);
    plain2array(y3, _y3);

    return result;
}

// Only the second operand is converted to Montgomery form, and the product uses Karatsuba,
// see complex_plain.hpp
int BLS12_381ComplexMulP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawBLS12_381_384::Element x1, y1, x2, y2, x3, y3;
    array2plain(p1, x1);
    array2plain(p1 + 6, y1);
    array2fe(p2, x2);
    array2fe(p2 + 6, y2);

    complex_mul_plain(bls12_381, x1, y1, x2, y2, x3, y3);
    int result = 0;

    plain2array(x3, p3);
    plain2array(y3, p3 + 6);

    return result;
}

/**************************/
/* BLS12_381 complex sqrt */
/**************************/

int BLS12_381ComplexSqrt (
    const uint64_t * _x1, // 6 x 64 bits
    const uint64_t * _y1, // 6 x 64 bits
    uint64_t * _x2, // 6 x 64 bits
    uint64_t * _y2, // 6 x 64 bits
    uint64_t * is_qr // 1 x 64 bits
)
{
    RawBLS12_381_384::Element x1, y1, x2, y2;
    array2fe(_x1, x1);
    array2fe(_y1, y1);

    int result = BLS12_381ComplexSqrtFe (x1, y1, x2, y2, *is_qr);

    fe2array(x2, _x2);
    fe2array(y2, _y2);

    return result;
}

int BLS12_381ComplexSqrtP (
    const uint64_t * p1, // 12 x 64 bits
    uint64_t * p2,  // 12 x 64 bits
    uint64_t * is_qr // 1 x 64 bits
)
{
    RawBLS12_381_384::Element x1, y1, x2, y2;
    array2fe(p1, x1);
    array2fe(p1 + 6, y1);

    int result = BLS12_381ComplexSqrtFe (x1, y1, x2, y2, *is_qr);

    fe2array(x2, p2);
    fe2array(y2, p2 + 6);

    return result;
}

#ifdef __cplusplus
} // extern "C"
#endif