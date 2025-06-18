#ifndef BN254_FE_HPP
#define BN254_FE_HPP

#include <stdint.h>
#include "../ffiasm/fq.hpp"
#include "../common/globals.hpp"

#ifdef __cplusplus
extern "C" {
#endif

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

int inline BN254ComplexAddFe (const RawFq::Element &x1, const RawFq::Element &y1, const RawFq::Element &x2, const RawFq::Element &y2, RawFq::Element &x3, RawFq::Element &y3)
{
    // Addition of 2 complex numbers:
    // x3 = x1 + x2 -> real parts are added
    // y3 = y1 + y2 -> imaginary parts are added

    bn254.add(x3, x1, x2);
    bn254.add(y3, y1, y2);

    return 0;
}

int inline BN254ComplexSubFe (const RawFq::Element &x1, const RawFq::Element &y1, const RawFq::Element &x2, const RawFq::Element &y2, RawFq::Element &x3, RawFq::Element &y3)
{
    // Subtraction of 2 complex numbers:
    // x3 = x1 - x2 -> real parts are subtracted
    // y3 = y1 - y2 -> imaginary parts are subtracted
    
    bn254.sub(x3, x1, x2);
    bn254.sub(y3, y1, y2);

    return 0;
}

int inline BN254ComplexMulFe (const RawFq::Element &x1, const RawFq::Element &y1, const RawFq::Element &x2, const RawFq::Element &y2, RawFq::Element &x3, RawFq::Element &y3)
{
    // Multiplication of 2 complex numbers:
    // x3 = x1 * x2 - y1 * y2 -> real parts are multiplied, minus the multiplication of the imaginary parts (i*i = -1)
    // y3 = y1 * x2 + x1 * y2 -> imaginary parts are multiplied by the opposite real parts

    RawFq::Element aux1, aux2, x3_temp;

    bn254.mul(aux1, x1, x2);
    bn254.mul(aux2, y1, y2);
    bn254.sub(x3_temp, aux1, aux2);
    
    bn254.mul(aux1, y1, x2);
    bn254.mul(aux2, x1, y2);
    bn254.add(y3, aux1, aux2);

    x3 = x3_temp;

    return 0;
}
int inline BN254ComplexInvFe (const RawFq::Element &real, const RawFq::Element &imaginary, RawFq::Element &inverse_real, RawFq::Element &inverse_imaginary)
{
    // Calculate denominator = real^2 + imaginary^2
    RawFq::Element denominator, aux;
    bn254.mul(denominator, real, real);
    bn254.mul(aux, imaginary, imaginary);
    bn254.add(denominator, denominator, aux);

    // Inverse the denominator to multiply it later
    bn254.inv(denominator, denominator);

    // inverse_real = real/denominator = real*inverse_denominator
    bn254.mul(inverse_real, real, denominator);

    // inverse_imaginary = -imaginary/denominator = -imaginary*inverse_denominator
    bn254.neg(aux, imaginary);
    bn254.mul(inverse_imaginary, aux, denominator);

    return 0;
};

#ifdef __cplusplus
} // extern "C"
#endif

#endif