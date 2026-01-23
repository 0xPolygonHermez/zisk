#include "fcall.hpp"
#include "../common/utils.hpp"
#include "../bn254/bn254_fe.hpp"
#include "../bls12_381/bls12_381_fe.hpp"
#include "../bls12_381/bls12_381.hpp"
#include "../ec/ec.hpp"
#include <stdint.h>
#include <assert.h>

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
        case FCALL_BLS12_381_FP_INV_ID:
        {
            iresult = BLS12_381FpInvCtx(ctx);
            break;
        }
        case FCALL_BLS12_381_FP_SQRT_ID:
        {
            iresult = BLS12_381FpSqrtCtx(ctx);
            break;
        }
        case FCALL_BLS12_381_FP2_INV_ID:
        {
            iresult = BLS12_381ComplexInvCtx(ctx);
            break;
        }
        case FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID:
        {
            iresult = BLS12_381TwistAddLineCoeffsCtx(ctx);
            break;
        }
        case FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID:
        {
            iresult = BLS12_381TwistDblLineCoeffsCtx(ctx);
            break;
        }
        case FCALL_MSB_POS_384_ID:
        {
            iresult = MsbPos384Ctx(ctx);
            break;
        }
        case FCALL_BIGINT256_DIV_ID:
        {
            iresult = BigInt256DivCtx(ctx);
            break;
        }
        case FCALL_BIG_INT_DIV_ID:
        {
            iresult = BigIntDivCtx(ctx);
            break;
        }
        case FCALL_BIN_DECOMP_ID:
        {
            iresult = BinDecompCtx(ctx);
            break;
        }
        case FCALL_BLS12_381_FP2_SQRT_ID:
        {
            iresult = BLS12_381Fp2SqrtCtx(ctx);
            break;
        }
        case FCALL_SECP256K1_ECDSA_VERIFY_ID:
        {
            iresult = Secp256k1EcdsaVerifyCtx(ctx);
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
        auxa = (auxa * 3) % p;
        mpz_powm(r.get_mpz_t(), auxa.get_mpz_t(), n.get_mpz_t(), p.get_mpz_t());
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
    const uint64_t n = a[0]; // number of inputs
    const uint64_t * params = &a[1];

    for (int limb=3; limb>=0; limb--)
    {
        // Find max value at this limb position across all inputs
        uint64_t max_word = 0;
        for (uint64_t i=0; i<n; i++)
        {
            uint64_t word = params[i * 4 + limb];
            if (word > max_word) {
                max_word = word;
            }
        }
        if (max_word != 0)
        {
            r[0] = limb;
            r[1] = msb_pos(max_word);
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

/***************************/
/* BLS12_381 CURVE INVERSE */
/***************************/

int BLS12_381FpInv (
    const uint64_t * _a, // 6 x 64 bits
          uint64_t * _r  // 6 x 64 bits
)
{
    RawBLS12_381_384::Element a;
    array2fe(_a, a);
    if (bls12_381.isZero(a))
    {
        printf("BLS12_381FpInv() Division by zero\n");
        return -1;
    }

    RawBLS12_381_384::Element r;
    bls12_381.inv(r, a);

    fe2array(r, _r);

    return 0;
}

int BLS12_381FpInvCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BLS12_381FpInv(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 6;
        ctx->result_size = 6;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/*******************************/
/* BLS12_381 CURVE SQUARE ROOT */
/*******************************/

int BLS12_381FpSqrt (
    const uint64_t * _a, // 6 x 64 bits
          uint64_t * _r  // 6 x 64 bits
)
{
    mpz_class a;
    array2scalar6(_a, a);

    // Attempt to compute the square root of a
    mpz_class r;
    mpz_powm(r.get_mpz_t(), a.get_mpz_t(), ScalarP_DIV_4.get_mpz_t(), ScalarP.get_mpz_t());

    // Check if a is a quadratic residue
    mpz_class square = (r * r) % ScalarP;
    uint64_t a_is_gr = (square == a) ? 1 : 0;
    _r[0] = a_is_gr;
    if (!a_is_gr)
    {
        // To check that a is indeed a non-quadratic residue, we check that
        // a * NQR is a quadratic residue for some fixed known non-quadratic residue NQR
        mpz_class a_nqr = (a * ScalarNQR_FP) % ScalarP;

        // Compute the square root of a * NQR
        mpz_powm(r.get_mpz_t(), a_nqr.get_mpz_t(), ScalarP_DIV_4.get_mpz_t(), ScalarP.get_mpz_t());
    }

    scalar2array6(r, &_r[1]);

    return 0;
}

int BLS12_381FpSqrtCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BLS12_381FpSqrt(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 7;
        ctx->result_size = 7;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/*****************************/
/* BLS12_381 COMPLEX INVERSE */
/*****************************/

// Inverse of a complex number a + ib is (a - ib) / (aa + bb):
// (a + ib) * (a - ib) / (aa + bb) = (aa + iab - iab - iibb) / (aa + bb) = (aa + bb) / (aa + bb) = 1

int BLS12_381ComplexInv (
    const uint64_t * a, // 12 x 64 bits
          uint64_t * r  // 12 x 64 bits
)
{
    // There is no need to check for 0 since this must be done at the rust level

    // Convert to field elements
    RawBLS12_381_384::Element real, imaginary;
    array2fe(a, real);
    array2fe(a + 6, imaginary);

    RawBLS12_381_384::Element r_real, r_imaginary;
    BLS12_381ComplexInvFe(real, imaginary, r_real, r_imaginary);

    fe2array(r_real, r);
    fe2array(r_imaginary, r + 6);

    return 0;
}

int BLS12_381ComplexInvCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BLS12_381ComplexInv(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 12;
        ctx->result_size = 12;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/***********************************/
/* BLS12_381 TWIST ADD LINE COEFFS */
/***********************************/

int BLS12_381TwistAddLineCoeffs (
    const uint64_t * a, // 48 x 64 bits
          uint64_t * r  // 24 x 64 bits
)
{
    // Convert to field elements
    RawBLS12_381_384::Element x1_real, x1_imaginary, y1_real, y1_imaginary, x2_real, x2_imaginary, y2_real, y2_imaginary;
    array2fe(a, x1_real);
    array2fe(a + 6, x1_imaginary);
    array2fe(a + 12, y1_real);
    array2fe(a + 18, y1_imaginary);
    array2fe(a + 24, x2_real);
    array2fe(a + 30, x2_imaginary);
    array2fe(a + 36, y2_real);
    array2fe(a + 42, y2_imaginary);

    // Compute 𝜆 = (y2 - y1)/(x2 - x1)
    RawBLS12_381_384::Element lambda_real, lambda_imaginary, aux_real, aux_imaginary;
    BLS12_381ComplexSubFe(x2_real, x2_imaginary, x1_real, x1_imaginary, lambda_real, lambda_imaginary); // 𝜆 = (x2 - x1)
    BLS12_381ComplexInvFe(lambda_real, lambda_imaginary, lambda_real, lambda_imaginary); // 𝜆 = 1/(x2 - x1)
    BLS12_381ComplexSubFe(y2_real, y2_imaginary, y1_real, y1_imaginary, aux_real, aux_imaginary); // aux = (y2 - y1)
    BLS12_381ComplexMulFe(lambda_real, lambda_imaginary, aux_real, aux_imaginary, lambda_real, lambda_imaginary); // 𝜆 = aux*𝜆 = (y2 - y1)/(x2 - x1)

    // Compute 𝜇 = y - 𝜆x
    RawBLS12_381_384::Element mu_real, mu_imaginary;
    BLS12_381ComplexMulFe(lambda_real, lambda_imaginary, x1_real, x1_imaginary, aux_real, aux_imaginary); // aux = 𝜆 - x1
    BLS12_381ComplexSubFe(y1_real, y1_imaginary, aux_real, aux_imaginary, mu_real, mu_imaginary); // 𝜇 = y1 - aux = y1 - 𝜆x1

    // Store the result
    fe2array(lambda_real, r);
    fe2array(lambda_imaginary, r + 6);
    fe2array(mu_real, r + 12);
    fe2array(mu_imaginary, r + 18);

    return 0;
}

int BLS12_381TwistAddLineCoeffsCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BLS12_381TwistAddLineCoeffs(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 24;
        ctx->result_size = 24;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/**************************************/
/* BLS12_381 TWIST DOUBLE LINE COEFFS */
/**************************************/

int BLS12_381TwistDblLineCoeffs (
    const uint64_t * a, // 24 x 64 bits
          uint64_t * r  // 24 x 64 bits
)
{
    // Convert to field elements
    RawBLS12_381_384::Element x_real, x_imaginary, y_real, y_imaginary;
    array2fe(a, x_real);
    array2fe(a + 6, x_imaginary);
    array2fe(a + 12, y_real);
    array2fe(a + 18, y_imaginary);


    // Compute 𝜆 = 3x²/2y
    RawBLS12_381_384::Element lambda_real, lambda_imaginary, aux_real, aux_imaginary, three;
    BLS12_381ComplexAddFe(y_real, y_imaginary, y_real, y_imaginary, lambda_real, lambda_imaginary); // 𝜆 = 2y
    BLS12_381ComplexInvFe(lambda_real, lambda_imaginary, lambda_real, lambda_imaginary); // 𝜆 = 1/2y
    BLS12_381ComplexMulFe(x_real, x_imaginary, x_real, x_imaginary, aux_real, aux_imaginary); // aux = x²
    BLS12_381ComplexMulFe(lambda_real, lambda_imaginary, aux_real, aux_imaginary, lambda_real, lambda_imaginary); // 𝜆 = x²/2y
    bls12_381.fromUI(three, 3); // 𝜆 = 3x²/2y
    bls12_381.mul(lambda_real, lambda_real, three);
    bls12_381.mul(lambda_imaginary, lambda_imaginary, three);

    // Compute 𝜇 = y - 𝜆x
    RawBLS12_381_384::Element mu_real, mu_imaginary;
    BLS12_381ComplexMulFe(lambda_real, lambda_imaginary, x_real, x_imaginary, aux_real, aux_imaginary); // aux = 𝜆x
    BLS12_381ComplexSubFe(y_real, y_imaginary, aux_real, aux_imaginary, mu_real, mu_imaginary); // 𝜇 = y - 𝜆x

    // Store the result
    fe2array(lambda_real, r);
    fe2array(lambda_imaginary, r + 6);
    fe2array(mu_real, r + 12);
    fe2array(mu_imaginary, r + 18);

    return 0;
}

int BLS12_381TwistDblLineCoeffsCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BLS12_381TwistDblLineCoeffs(ctx->params, ctx->result);
    if (iresult == 0)
    {
        iresult = 24;
        ctx->result_size = 24;
    }
    else
    {
        ctx->result_size = 0;
    }
    return iresult;
}

/***************/
/* MSB POS 384 */
/***************/

int MsbPos384 (
    const uint64_t * a, // 12 x 64 bits
          uint64_t * r  // 2 x 64 bits
)
{
    const uint64_t * x = a;
    const uint64_t * y = &a[6];

    for (int i=5; i>=0; i--)
    {
        if ((x[i] != 0) || (y[i] != 0))
        {
            uint64_t word = x[i] > y[i] ? x[i] : y[i];
            r[0] = i;
            r[1] = msb_pos(word);
            return 0;
        }
    }
    printf("MsbPos384() error: both x and y are zero\n");
    exit(-1);
}

int MsbPos384Ctx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = MsbPos384(ctx->params, ctx->result);
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

/*************************************/
/* BIT INT 256 DIVISION AND REMINDER */
/*************************************/

int BigInt256Div (
    const uint64_t * _a, // 8 x 64 bits
          uint64_t * _r  // 8 x 64 bits
)
{
    mpz_class a, b;
    array2scalar(_a, a);
    array2scalar(_a + 4, b);

    mpz_class quotient = a / b;
    mpz_class remainder = a % b;

    scalar2array(quotient, &_r[0]);
    scalar2array(remainder, &_r[4]);

    return 0;
}

int BigInt256DivCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int iresult = BigInt256Div(ctx->params, ctx->result);
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

/********************/
/* BIG INT DIVISION */
/********************/

int BigIntDivCtx (
    struct FcallContext * ctx  // fcall context
)
{
    // Parse input parameters lengths
    uint64_t len_a = ctx->params[0];
    assert(len_a < FCALL_PARAMS_MAX_SIZE);
    uint64_t len_b = ctx->params[1 + len_a];
    assert(len_b < FCALL_PARAMS_MAX_SIZE);

    // Convert first parameter to mpz_class
    mpz_class a;
    mpz_import(a.get_mpz_t(), len_a, -1, 8, -1, 0, (const void *)&ctx->params[1]);

    // Convert second parameter to mpz_class
    mpz_class b;
    mpz_import(b.get_mpz_t(), len_b, -1, 8, -1, 0, (const void *)&ctx->params[2 + len_a]);

    // Compute quotient and remainder
    mpz_class quotient = a / b;
    mpz_class remainder = a % b;

    // Convert quotient to an array of u64 starting at ctx->result[1], with length multiple of 4
    size_t exported_size = 0;
    mpz_export((void *)&ctx->result[1], &exported_size, -1, 8, -1, 0, quotient.get_mpz_t());
    size_t quotient_size = (exported_size == 0) ? 4 : ((exported_size + 3)/4)*4;
    ctx->result[0] = quotient_size;
    for (size_t i=exported_size; i<quotient_size; i++)
    {
        ctx->result[1 + i] = 0;
    }

    // Convert remainder to an array of u64 starting at ctx->result[2 + quotient_size],
    // with length multiple of 4
    mpz_export((void *)&ctx->result[2 + quotient_size], &exported_size, -1, 8, -1, 0, remainder.get_mpz_t());
    size_t remainder_size = (exported_size == 0) ? 4 : ((exported_size + 3)/4)*4;
    ctx->result[1 + quotient_size] = remainder_size;
    for (size_t i=exported_size; i<remainder_size; i++)
    {
        ctx->result[2 + quotient_size + i] = 0;
    }

    uint64_t total_size = 2 + quotient_size + remainder_size;
    assert(total_size < FCALL_RESULT_MAX_SIZE);

    ctx->result_size = total_size;

    return total_size;
}

/************************/
/* BINARY DECOMPOSITION */
/************************/

int BinDecompCtx (
    struct FcallContext * ctx  // fcall context
)
{
    // Parse input parameter length
    uint64_t len_x = ctx->params[0];
    assert(len_x < FCALL_PARAMS_MAX_SIZE);

    // Perform binary decomposition
    ctx->result_size = 0;
    bool started = false;

    // For every u64 in the input parameter, in reverse order
    for (int i = len_x - 1; i >= 0; i--)
    {
        // For every bit in the u64, in reverse order
        for (int bit_pos = 63; bit_pos >= 0; bit_pos--)
        {
            // Obtain the bit value
            uint8_t bit = (ctx->params[1 + i] >> bit_pos) & 1;

            // Start recording once we hit the first 1 bit
            if (!started && bit == 1)
            {
                started = true;
            }

            // If started, record the bit
            if (started)
            {
                ctx->result[1 + ctx->result_size] = bit;
                ctx->result_size += 1;
                assert(ctx->result_size < FCALL_RESULT_MAX_SIZE);
            }
        }
    }

    // Store the result size at the beginning of the result array
    ctx->result[0] = ctx->result_size;
    ctx->result_size++;
    
    return 0;
}

/**********************/
/* BLS12 381 FP2 SQRT */
/**********************/

uint64_t NQR[12] = {1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0};

/// Computes the square root of a non-zero field element in Fp2
int BLS12_381Fp2SqrtCtx (
    struct FcallContext * ctx  // fcall context
)
{
    int result;

    // Perform the square root
    result = BLS12_381ComplexSqrtP(
        &ctx->params[0], // 12 x 64 bits input parameter: real(6) + imaginary(6)
        &ctx->result[1], // 12 x 64 bits output parameter: real(6) + imaginary(6)
        &ctx->result[0]  // 1 x 64 bits output parameter: is_quadratic_residue (1)
    );
    if (result != 0) return result;

    // Check if a is a quadratic residue
    if (!ctx->result[0])
    {
        // To check that a is indeed a non-quadratic residue, we check that
        // a * NQR is a quadratic residue for some fixed known non-quadratic residue NQR
        uint64_t a_nqr[12];
        result = BLS12_381ComplexMulP(
            &ctx->params[0], // 12 x 64 bits input parameter: real(6) + imaginary(6)
            &NQR[0], // 12 x 64 bits input parameter: real(6) + imaginary(6)
            &a_nqr[0] // 12 x 64 bits output parameter: real(6) + imaginary(6)
        );
        if (result != 0) return result;

        // Compute the square root of a * NQR
        uint64_t aux; // Unused
        result = BLS12_381ComplexSqrtP(
            &a_nqr[0], // 12 x 64 bits input parameter: real(6) + imaginary(6)
            &ctx->result[1], // 12 x 64 bits output parameter: real(6) + imaginary(6)
            &aux  // 1 x 64 bits output parameter: is_quadratic_residue (1)
        );
        if (result != 0) return result;
    }

    ctx->result_size = 13;
    
    return 0;
}

int Secp256k1EcdsaVerifyCtx(
    struct FcallContext * ctx  // fcall context
)
{
    secp256k1_ecdsa_verify( &ctx->params[0], &ctx->params[8], &ctx->params[12], &ctx->params[16], &ctx->result[0]);
    ctx->result_size = 8;
    return 0;
}