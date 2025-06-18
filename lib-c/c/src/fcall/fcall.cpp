#include "fcall.hpp"
#include "../common/utils.hpp"
#include "../bn254/bn254_fe.hpp"
#include <stdint.h>

int Fcall (
    struct FcallContext * ctx  // fcall context
)
{
    // Switch based on function id
    int iresult;
    switch (ctx->function_id)
    {
        case FCALL_ID_INVERSE_FP_EC:
        {
            iresult = InverseFpEcCtx(ctx);
            break;
        }
        case FCALL_ID_INVERSE_FN_EC:
        {
            iresult = InverseFnEcCtx(ctx);
            break;
        }
        case FCALL_ID_SQRT_FP_EC_PARITY:
        {
            iresult = SqrtFpEcParityCtx(ctx);
            break;
        }
        case FCALL_ID_MSB_POS_256:
        {
            iresult = MsbPos256Ctx(ctx);
            break;
        }
        case FCALL_ID_BN254_FP_INV:
        {
            iresult = BN254FpInvCtx(ctx);
            break;
        }
        case FCALL_ID_BN254_FP2_INV:
        {
            iresult = BN254ComplexInvCtx(ctx);
            break;
        }
        case FCALL_ID_BN254_TWIST_ADD_LINE_COEFFS:
        {
            iresult = BN254TwistAddLineCoeffsCtx(ctx);
            break;
        }
        case FCALL_ID_BN254_TWIST_DBL_LINE_COEFFS:
        {
            iresult = BN254TwistDblLineCoeffsCtx(ctx);
            break;
        }
        default:
        {
            printf("Fcall() found unsupported function_id=%lu\n", ctx->function_id);
            return -1;
        }
    }

    return iresult;
}

/***************/
/* INVERSE FEC */
/***************/

int InverseFpEc (
    const uint64_t * _a, // 4 x 64 bits
          uint64_t * _r  // 4 x 64 bits
)
{
    // TODO: call mpz_invert
    RawFec::Element a;
    array2fe(_a, a);
    if (fec.isZero(a))
    {
        printf("InverseFpEc() Division by zero\n");
        return -1;
    }

    RawFec::Element r;
    fec.inv(r, a);

    fe2array(r, _r);

    return 0;
}

int InverseFpEcCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = InverseFpEc(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 4;
        ctx->result_size = 4;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/****************/
/* INVERSE FNEC */
/****************/

int InverseFnEc (
    const uint64_t * _a,  // 8 x 64 bits
    uint64_t * _r  // 8 x 64 bits
)
{
    RawFnec::Element a;
    array2fe(_a, a);
    if (fnec.isZero(a))
    {
        printf("InverseFnEc() Division by zero\n");
        return -1;
    }

    RawFnec::Element r;
    fnec.inv(r, a);

    fe2array(r, _r);

    return 0;
}

int InverseFnEcCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = InverseFnEc(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 4;
        ctx->result_size = 4;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/************/
/* FEC SQRT */
/************/

mpz_class n("0x3fffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffff0c");
mpz_class p("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F");

// We use that p = 3 mod 4 => r = a^((p+1)/4) is a square root of a
// https://www.rieselprime.de/ziki/Modular_square_root
// n = p+1/4
// return true if sqrt exists, false otherwise

inline bool sqrtF3mod4(mpz_class &r, const mpz_class &a)
{
    mpz_class auxa = a;
    mpz_powm(r.get_mpz_t(), a.get_mpz_t(), n.get_mpz_t(), p.get_mpz_t());
    if ((r * r) % p != auxa)
    {
        r = ScalarMask256;
        return false;
    }
    return true;
}

int SqrtFpEcParity (
    const uint64_t * _a,  // 4 x 64 bits
    const uint64_t _parity,  // 1 x 64 bits
    uint64_t * _r  // 1 x 64 bits (sqrt exists) + 4 x 64 bits
)
{
    mpz_class parity(static_cast<unsigned long>(_parity));
    mpz_class a;
    array2scalar(_a, a);

    // Call the sqrt function
    mpz_class r;
    bool sqrt_exists = sqrtF3mod4(r, a);

    _r[0] = sqrt_exists;

    // Post-process the result
    if (r == ScalarMask256)
    {
        // This sqrt does not have a solution
    }
    else if ((r & 1) == parity)
    {
        // Return r as it is, since it has the requested parity
    }
    else
    {
        // Negate the result
        RawFec::Element fe;
        fec.fromMpz(fe, r.get_mpz_t());
        fe = fec.neg(fe);
        fec.toMpz(r.get_mpz_t(), fe);
    }

    scalar2array(r, &_r[1]);

    return 0;
}

int SqrtFpEcParityCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = SqrtFpEcParity(ctx->params, ctx->params[4], &ctx->result[0]);
    if (iresult == 0)
    {
        iresult = 5;
        ctx->result_size = 5;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/***************/
/* MSB POS 256 */
/***************/

uint64_t msb_pos(uint64_t x)
{
    uint64_t pos = 0;
    if (x >= (1UL << 32)) { x >>= 32; pos += 32; }
    if (x >= (1UL << 16)) { x >>= 16; pos += 16; }
    if (x >= (1UL << 8 )) { x >>= 8;  pos += 8;  }
    if (x >= (1UL << 4 )) { x >>= 4;  pos += 4;  }
    if (x >= (1UL << 2 )) { x >>= 2;  pos += 2;  }
    if (x >= (1UL << 1 )) {           pos += 1;  }
    return pos;
}

int MsbPos256 (
    const uint64_t * a, // 8 x 64 bits
          uint64_t * r  // 2 x 64 bits
)
{
    const uint64_t * x = a;
    const uint64_t * y = &a[4];

    for (int i=3; i>=0; i--)
    {
        if ((x[i] != 0) || (y[i] != 0))
        {
            uint64_t word = x[i] > y[i] ? x[i] : y[i];
            r[0] = i;
            r[1] = msb_pos(word);
            return 0;
        }
    }
    printf("MsbPos256() error: both x and y are zero\n");
    exit(-1);
}

int MsbPos256Ctx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = MsbPos256(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 2;
        ctx->result_size = 2;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/***********************/
/* BN254 CURVE INVERSE */
/***********************/

int BN254FpInv (
    const uint64_t * _a, // 4 x 64 bits
          uint64_t * _r  // 4 x 64 bits
)
{
    RawFq::Element a;
    array2fe(_a, a);
    if (bn254.isZero(a))
    {
        printf("BN254FpInv() Division by zero\n");
        return -1;
    }

    RawFq::Element r;
    bn254.inv(r, a);

    fe2array(r, _r);

    return 0;
}

int BN254FpInvCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BN254FpInv(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 4;
        ctx->result_size = 4;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/*************************/
/* BN254 COMPLEX INVERSE */
/*************************/

// Inverse of a complex number a + ib is (a - ib) / (aa + bb):
// (a + ib) * (a - ib) / (aa + bb) = (aa + iab - iab - iibb) / (aa + bb) = (aa + bb) / (aa + bb) = 1

int BN254ComplexInv (
    const uint64_t * a, // 8 x 64 bits
          uint64_t * r  // 8 x 64 bits
)
{
    // There is no need to check for 0 since this must be done at the rust level

    // Convert to field elements
    RawFq::Element real, imaginary;
    array2fe(a, real);
    array2fe(a + 4, imaginary);

    RawFq::Element r_real, r_imaginary;
    BN254ComplexInvFe(real, imaginary, r_real, r_imaginary);

    fe2array(r_real, r);
    fe2array(r_imaginary, r + 4);

    return 0;
}

int BN254ComplexInvCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BN254ComplexInv(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 8;
        ctx->result_size = 8;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/*******************************/
/* BN254 TWIST ADD LINE COEFFS */
/*******************************/

int BN254TwistAddLineCoeffs (
    const uint64_t * a, // 32 x 64 bits
          uint64_t * r  // 16 x 64 bits
)
{
    // Convert to field elements
    RawFq::Element x1_real, x1_imaginary, y1_real, y1_imaginary, x2_real, x2_imaginary, y2_real, y2_imaginary;
    array2fe(a, x1_real);
    array2fe(a + 4, x1_imaginary);
    array2fe(a + 8, y1_real);
    array2fe(a + 12, y1_imaginary);
    array2fe(a + 16, x2_real);
    array2fe(a + 20, x2_imaginary);
    array2fe(a + 24, y2_real);
    array2fe(a + 28, y2_imaginary);

    // Compute 𝜆 = (y2 - y1)/(x2 - x1)
    RawFq::Element lambda_real, lambda_imaginary, aux_real, aux_imaginary;
    BN254ComplexSubFe(x2_real, x2_imaginary, x1_real, x1_imaginary, lambda_real, lambda_imaginary); // 𝜆 = (x2 - x1)
    BN254ComplexInvFe(lambda_real, lambda_imaginary, lambda_real, lambda_imaginary); // 𝜆 = 1/(x2 - x1)
    BN254ComplexSubFe(y2_real, y2_imaginary, y1_real, y1_imaginary, aux_real, aux_imaginary); // aux = (y2 - y1)
    BN254ComplexMulFe(lambda_real, lambda_imaginary, aux_real, aux_imaginary, lambda_real, lambda_imaginary); // 𝜆 = aux*𝜆 = (y2 - y1)/(x2 - x1)

    // Compute 𝜇 = y - 𝜆x
    RawFq::Element mu_real, mu_imaginary;
    BN254ComplexMulFe(lambda_real, lambda_imaginary, x1_real, x1_imaginary, aux_real, aux_imaginary); // aux = 𝜆 - x1
    BN254ComplexSubFe(y1_real, y1_imaginary, aux_real, aux_imaginary, mu_real, mu_imaginary); // 𝜇 = y1 - aux = y1 - 𝜆x1

    // Store the result
    fe2array(lambda_real, r);
    fe2array(lambda_imaginary, r + 4);
    fe2array(mu_real, r + 8);
    fe2array(mu_imaginary, r + 12);

    return 0;
}

int BN254TwistAddLineCoeffsCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BN254TwistAddLineCoeffs(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 16;
        ctx->result_size = 16;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/**********************************/
/* BN254 TWIST DOUBLE LINE COEFFS */
/**********************************/

int BN254TwistDblLineCoeffs (
    const uint64_t * a, // 32 x 64 bits
          uint64_t * r  // 16 x 64 bits
)
{
    // Convert to field elements
    RawFq::Element x_real, x_imaginary, y_real, y_imaginary;
    array2fe(a, x_real);
    array2fe(a + 4, x_imaginary);
    array2fe(a + 8, y_real);
    array2fe(a + 12, y_imaginary);


    // Compute 𝜆 = 3x²/2y
    RawFq::Element lambda_real, lambda_imaginary, aux_real, aux_imaginary, three;
    BN254ComplexAddFe(y_real, y_imaginary, y_real, y_imaginary, lambda_real, lambda_imaginary); // 𝜆 = 2y
    BN254ComplexInvFe(lambda_real, lambda_imaginary, lambda_real, lambda_imaginary); // 𝜆 = 1/2y
    BN254ComplexMulFe(x_real, x_imaginary, x_real, x_imaginary, aux_real, aux_imaginary); // aux = x²
    BN254ComplexMulFe(lambda_real, lambda_imaginary, aux_real, aux_imaginary, lambda_real, lambda_imaginary); // 𝜆 = x²/2y
    bn254.fromUI(three, 3); // 𝜆 = 3x²/2y
    bn254.mul(lambda_real, lambda_real, three);
    bn254.mul(lambda_imaginary, lambda_imaginary, three);

    // Compute 𝜇 = y - 𝜆x
    RawFq::Element mu_real, mu_imaginary;
    BN254ComplexMulFe(lambda_real, lambda_imaginary, x_real, x_imaginary, aux_real, aux_imaginary); // aux = 𝜆x
    BN254ComplexSubFe(y_real, y_imaginary, aux_real, aux_imaginary, mu_real, mu_imaginary); // 𝜇 = y - 𝜆x

    // Store the result
    fe2array(lambda_real, r);
    fe2array(lambda_imaginary, r + 4);
    fe2array(mu_real, r + 8);
    fe2array(mu_imaginary, r + 12);

    return 0;
}

int BN254TwistDblLineCoeffsCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BN254TwistDblLineCoeffs(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 16;
        ctx->result_size = 16;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}