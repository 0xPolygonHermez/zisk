#include "fcall.hpp"
#include "../common/utils.hpp"

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
    const unsigned long * _a, // 4 x 64 bits
          unsigned long * _r  // 4 x 64 bits
)
{
    // TODO: call mpz_invert
    printf("InverseFpEc() _a[0]=%0X\n", _a[0]);
    printf("InverseFpEc() _a[1]=%0X\n", _a[1]);
    printf("InverseFpEc() _a[2]=%0X\n", _a[2]);
    printf("InverseFpEc() _a[3]=%0X\n", _a[3]);
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
    const unsigned long * _a,  // 8 x 64 bits
    unsigned long * _r  // 8 x 64 bits
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
    const unsigned long * _a,  // 4 x 64 bits
    const unsigned long _parity,  // 1 x 64 bits
    unsigned long * _r  // 1 x 64 bits (sqrt exists) + 4 x 64 bits
)
{
    mpz_class parity = _parity;
    gmp_printf("parity: %Zx\n", parity);
    mpz_class a;
    array2scalar(_a, a);
    gmp_printf("a: %Zx\n", a);

    // Call the sqrt function
    mpz_class r;
    bool sqrt_exists = sqrtF3mod4(r, a);
    printf("sqrt_exists: %d\n", sqrt_exists);
    gmp_printf("r: %Zx\n", r);

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
    gmp_printf("r: %Zx\n", r);
    printf("_r: [%llx,%llx,%llx,%llx]\n", _r[1], _r[2], _r[3], _r[4]);

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
