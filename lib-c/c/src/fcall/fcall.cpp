#include "fcall.hpp"
#include "../common/utils.hpp"
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
        default:
        {
            printf("Fcall() found unsupported function_id=%llu\n", ctx->function_id);
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
        iresult = 8;
        ctx->result_size = 8;
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
        iresult = 8;
        ctx->result_size = 8;
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
