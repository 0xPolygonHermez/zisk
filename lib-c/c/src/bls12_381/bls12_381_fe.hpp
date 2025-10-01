#ifndef BLS12_381_FE_HPP
#define BLS12_381_FE_HPP

#include <stdint.h>
#include "../ffiasm/bls12_381_384.hpp"
#include "../common/globals.hpp"

#ifdef __cplusplus
extern "C" {
#endif

int inline BLS12_381CurveAddFe (const RawBLS12_381_384::Element &x1, const RawBLS12_381_384::Element &y1, const RawBLS12_381_384::Element &x2, const RawBLS12_381_384::Element &y2, RawBLS12_381_384::Element &x3, RawBLS12_381_384::Element &y3)
{
    RawBLS12_381_384::Element aux1, aux2, s;

    // s = (y2-y1)/(x2-x1)
    bls12_381.sub(aux1, y2, y1);
    bls12_381.sub(aux2, x2, x1);
    if (bls12_381.isZero(aux2))
    {
        printf("BLS12_381CurveAddFe() got denominator=0 2\n");
        return -1;
    }
    bls12_381.div(s, aux1, aux2);

    // Required for x3 calculation
    bls12_381.add(aux2, x1, x2);

    // x3 = s*s - (x1+x2)
    bls12_381.mul(aux1, s, s);
    // aux2 was calculated before
    bls12_381.sub(x3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    bls12_381.sub(aux1, x1, x3);
    bls12_381.mul(aux1, aux1, s);
    bls12_381.sub(y3, aux1, y1);

    return 0;
}

int inline BLS12_381CurveDblFe (const RawBLS12_381_384::Element &x1, const RawBLS12_381_384::Element &y1, RawBLS12_381_384::Element &x2, RawBLS12_381_384::Element &y2)
{
    RawBLS12_381_384::Element aux1, aux2, s;

    // s = 3*x1*x1/2*y1
    bls12_381.mul(aux1, x1, x1);
    bls12_381.fromUI(aux2, 3);
    bls12_381.mul(aux1, aux1, aux2);
    bls12_381.add(aux2, y1, y1);
    if (bls12_381.isZero(aux2))
    {
        printf("BLS12_381CurveDblFe() got denominator=0 1\n");
        return -1;
    }
    bls12_381.div(s, aux1, aux2);

    // Required for x3 calculation
    bls12_381.add(aux2, x1, x1);

    // x2 = s*s - (x1+x2)
    bls12_381.mul(aux1, s, s);
    // aux2 was calculated before
    bls12_381.sub(x2, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    bls12_381.sub(aux1, x1, x2);
    bls12_381.mul(aux1, aux1, s);
    bls12_381.sub(y2, aux1, y1);

    return 0;
}

int inline BLS12_381ComplexAddFe (const RawBLS12_381_384::Element &x1, const RawBLS12_381_384::Element &y1, const RawBLS12_381_384::Element &x2, const RawBLS12_381_384::Element &y2, RawBLS12_381_384::Element &x3, RawBLS12_381_384::Element &y3)
{
    // Addition of 2 complex numbers:
    // x3 = x1 + x2 -> real parts are added
    // y3 = y1 + y2 -> imaginary parts are added

    bls12_381.add(x3, x1, x2);
    bls12_381.add(y3, y1, y2);

    return 0;
}

int inline BLS12_381ComplexSubFe (const RawBLS12_381_384::Element &x1, const RawBLS12_381_384::Element &y1, const RawBLS12_381_384::Element &x2, const RawBLS12_381_384::Element &y2, RawBLS12_381_384::Element &x3, RawBLS12_381_384::Element &y3)
{
    // Subtraction of 2 complex numbers:
    // x3 = x1 - x2 -> real parts are subtracted
    // y3 = y1 - y2 -> imaginary parts are subtracted

    bls12_381.sub(x3, x1, x2);
    bls12_381.sub(y3, y1, y2);

    return 0;
}

int inline BLS12_381ComplexMulFe (const RawBLS12_381_384::Element &x1, const RawBLS12_381_384::Element &y1, const RawBLS12_381_384::Element &x2, const RawBLS12_381_384::Element &y2, RawBLS12_381_384::Element &x3, RawBLS12_381_384::Element &y3)
{
    // Multiplication of 2 complex numbers:
    // x3 = x1 * x2 - y1 * y2 -> real parts are multiplied, minus the multiplication of the imaginary parts (i*i = -1)
    // y3 = y1 * x2 + x1 * y2 -> imaginary parts are multiplied by the opposite real parts

    RawBLS12_381_384::Element aux1, aux2, x3_temp;

    bls12_381.mul(aux1, x1, x2);
    bls12_381.mul(aux2, y1, y2);
    bls12_381.sub(x3_temp, aux1, aux2);

    bls12_381.mul(aux1, y1, x2);
    bls12_381.mul(aux2, x1, y2);
    bls12_381.add(y3, aux1, aux2);

    x3 = x3_temp;

    return 0;
}
int inline BLS12_381ComplexInvFe (const RawBLS12_381_384::Element &real, const RawBLS12_381_384::Element &imaginary, RawBLS12_381_384::Element &inverse_real, RawBLS12_381_384::Element &inverse_imaginary)
{
    // Calculate denominator = real^2 + imaginary^2
    RawBLS12_381_384::Element denominator, aux;
    bls12_381.mul(denominator, real, real);
    bls12_381.mul(aux, imaginary, imaginary);
    bls12_381.add(denominator, denominator, aux);

    // Inverse the denominator to multiply it later
    bls12_381.inv(denominator, denominator);

    // inverse_real = real/denominator = real*inverse_denominator
    bls12_381.mul(inverse_real, real, denominator);

    // inverse_imaginary = -imaginary/denominator = -imaginary*inverse_denominator
    bls12_381.neg(aux, imaginary);
    bls12_381.mul(inverse_imaginary, aux, denominator);

    return 0;
};

#ifdef __cplusplus
} // extern "C"
#endif

#endif