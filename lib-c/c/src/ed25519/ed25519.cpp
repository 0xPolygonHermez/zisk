#include <gmpxx.h>
#include <stdint.h>
#include <stdio.h>
#include "ed25519.hpp"
#include "../common/utils.hpp"

// Fp = 2^255 - 19 and d = -121665/121666 mod p. Plain GMP arithmetic: there is no ffiasm field
// for this prime.
static const mpz_class ED25519_P("7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffed", 16);
static const mpz_class ED25519_D("52036cee2b6ffe738cc740797779e89800700a4d4141d8ab75eb4dca135978a3", 16);

// a mod p, as a canonical non-negative value
static inline void ed25519_reduce (mpz_class &a)
{
    mpz_mod(a.get_mpz_t(), a.get_mpz_t(), ED25519_P.get_mpz_t());
}

// out = num / den mod p; returns -1 if den == 0 mod p
static inline int ed25519_div (mpz_class &out, const mpz_class &num, const mpz_class &den)
{
    mpz_class inv;
    if (mpz_invert(inv.get_mpz_t(), den.get_mpz_t(), ED25519_P.get_mpz_t()) == 0)
    {
        return -1;
    }
    out = num * inv;
    ed25519_reduce(out);
    return 0;
}

#ifdef __cplusplus
extern "C" {
#endif

int Ed25519AddP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    mpz_class x1, y1, x2, y2;
    array2scalar(p1, x1);
    array2scalar(p1 + 4, y1);
    array2scalar(p2, x2);
    array2scalar(p2 + 4, y2);
    ed25519_reduce(x1);
    ed25519_reduce(y1);
    ed25519_reduce(x2);
    ed25519_reduce(y2);

    mpz_class x1x2 = x1 * x2;
    mpz_class y1y2 = y1 * y2;
    ed25519_reduce(x1x2);
    ed25519_reduce(y1y2);

    // dtau = d*x1*x2*y1*y2
    mpz_class dtau = x1x2 * y1y2;
    ed25519_reduce(dtau);
    dtau *= ED25519_D;
    ed25519_reduce(dtau);

    mpz_class x3, y3;

    // x3 = (x1*y2 + y1*x2) / (1 + d*tau)
    mpz_class num = x1 * y2 + y1 * x2;
    mpz_class den = 1 + dtau;
    ed25519_reduce(num);
    ed25519_reduce(den);
    if (ed25519_div(x3, num, den) != 0)
    {
        printf("Ed25519AddP() got denominator=0 (1 + d*tau)\n");
        return -1;
    }

    // y3 = (y1*y2 + x1*x2) / (1 - d*tau)    (a = -1)
    num = y1y2 + x1x2;
    den = 1 - dtau;
    ed25519_reduce(num);
    ed25519_reduce(den);
    if (ed25519_div(y3, num, den) != 0)
    {
        printf("Ed25519AddP() got denominator=0 (1 - d*tau)\n");
        return -1;
    }

    scalar2array(x3, p3);
    scalar2array(y3, p3 + 4);
    return 0;
}

#ifdef __cplusplus
} // extern "C"
#endif
