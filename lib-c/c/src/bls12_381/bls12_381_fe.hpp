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

int inline BLS12_381ComplexExpFe (const RawBLS12_381_384::Element &x1, const RawBLS12_381_384::Element &y1, const mpz_class &_exp, RawBLS12_381_384::Element &x2, RawBLS12_381_384::Element &y2)
{
    // Exponentiation of a complex number using square-and-multiply algorithm

    // Get a local copy of the base to modify it
    RawBLS12_381_384::Element base_x, base_y;
    base_x = x1;
    base_y = y1;

    // Get a scalar copy of the exponent to modify it
    mpz_class exp(_exp);

    // Initialize result to 1 + 0i
    x2 = bls12_381.one(); // x2 = 1
    y2 = bls12_381.zero(); // y2 = 0

    // Loop until exponent becomes zero
    while (exp != 0)
    {
        // If exponent is odd, multiply the result by the base
        if ((exp & 1) == 1)
        {
            BLS12_381ComplexMulFe(x2, y2, base_x, base_y, x2, y2);
        }

        // Square the base
        BLS12_381ComplexMulFe(base_x, base_y, base_x, base_y, base_x, base_y);

        // Divide exponent by 2
        exp = exp >> 1;
    }
    
    return 0;
}

int inline BLS12_381ComplexSqrtFe (const RawBLS12_381_384::Element &x1, const RawBLS12_381_384::Element &y1, RawBLS12_381_384::Element &x2, RawBLS12_381_384::Element &y2, uint64_t &is_qr)
{
    /// Algorithm 9 from https://eprint.iacr.org/2012/685.pdf
    /// Square root computation over F_p^2, with p ≡ 3 (mod 4)

    // Step 1: a1 ← a^((p-3)/4)
    RawBLS12_381_384::Element a1_x, a1_y;
    BLS12_381ComplexExpFe(x1, y1, ScalarP_MINUS_3_DIV_4, a1_x, a1_y);

    // Step 2: α ← a1 * a1 * a
    RawBLS12_381_384::Element a1_a_x, a1_a_y;
    BLS12_381ComplexMulFe(a1_x, a1_y, x1, y1, a1_a_x, a1_a_y);
    RawBLS12_381_384::Element alpha_x, alpha_y;
    BLS12_381ComplexMulFe(a1_x, a1_y, a1_a_x, a1_a_y, alpha_x, alpha_y);

    // Step 3: a0 ← α^p * α = conjugate(α) * α
    RawBLS12_381_384::Element alpha_conj_x, alpha_conj_y;
    bls12_381.copy(alpha_conj_x, alpha_x);
    bls12_381.neg(alpha_conj_y, alpha_y);
    RawBLS12_381_384::Element a0_x, a0_y;
    BLS12_381ComplexMulFe(alpha_conj_x, alpha_conj_y, alpha_x, alpha_y, a0_x, a0_y);
    
    // Step 4-6: if a0 == -1 then return false (no square root)
    if (bls12_381.eq(a0_x, bls12_381.negOne()) && bls12_381.isZero(a0_y))
    {
        // Return false (no square root exists)
        is_qr = 0;
        x2 = bls12_381.zero();
        y2 = bls12_381.zero();
        return 0;
    }

    // Step 7: x0 ← a1 * a
    #define x0_x a1_a_x
    #define x0_y a1_a_y

    // Step 8-13: compute x based on α
    // If α == -1 then x ← i * x0 else x ← b * x0
    if (bls12_381.eq(a0_x, bls12_381.negOne()) && bls12_381.isZero(a0_y))
    {
        // Step 9: x ← i * x0
        BLS12_381ComplexMulFe(
            bls12_381.zero(), // i real part = 0
            bls12_381.one(), // i imaginary part = 1
            x0_x,
            x0_y,
            x2,
            y2
        );
    }
    else
    {
        // Step 11: b ← (1 + α)^((p-1)/2)
        RawBLS12_381_384::Element one_plus_alpha_x, one_plus_alpha_y;
        BLS12_381ComplexAddFe(
            bls12_381.one(), // 1 real part = 1
            bls12_381.zero(), // 1 imaginary part = 0
            alpha_x,
            alpha_y,
            one_plus_alpha_x,
            one_plus_alpha_y
        );
        RawBLS12_381_384::Element b_x, b_y;
        BLS12_381ComplexExpFe(one_plus_alpha_x, one_plus_alpha_y, ScalarP_MINUS_1_DIV_2, b_x, b_y);

        // Step 12: x ← b * x0
        BLS12_381ComplexMulFe(b_x, b_y, x0_x, x0_y, x2, y2);
    }

    // Return true (square root exists)
    is_qr = 1;

    return 0;
}

#ifdef __cplusplus
} // extern "C"
#endif

#endif