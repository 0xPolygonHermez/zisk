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

// (p + 3) / 8 and sqrt(-1) = 2^((p - 1) / 4)
static const mpz_class ED25519_P_PLUS_3_DIV_8 = (ED25519_P + 3) / 8;
static mpz_class ed25519_sqrt_m1 ()
{
    mpz_class r, e = (ED25519_P - 1) / 4, two = 2;
    mpz_powm(r.get_mpz_t(), two.get_mpz_t(), e.get_mpz_t(), ED25519_P.get_mpz_t());
    return r;
}
static const mpz_class ED25519_SQRT_M1 = ed25519_sqrt_m1();

// Square root mod p = 5 (mod 8): c = w^((p+3)/8) satisfies c^2 = w or c^2 = -w; in the second
// case c*sqrt(-1) is the root. Returns false if w is not a square.
static bool ed25519_sqrt (const mpz_class &w, mpz_class &root)
{
    mpz_class c, c2;
    mpz_powm(c.get_mpz_t(), w.get_mpz_t(), ED25519_P_PLUS_3_DIV_8.get_mpz_t(), ED25519_P.get_mpz_t());
    c2 = c * c;
    ed25519_reduce(c2);
    if (c2 == w)
    {
        root = c;
        return true;
    }
    mpz_class s = c2 + w;
    ed25519_reduce(s);
    if (s == 0)
    {
        root = c * ED25519_SQRT_M1;
        ed25519_reduce(root);
        return true;
    }
    return false;
}

#ifdef __cplusplus
extern "C" {
#endif

int Ed25519FpSqrtRatio (const uint64_t * _u, const uint64_t * _v, uint64_t * r)
{
    mpz_class u, v;
    array2scalar(_u, u);
    array2scalar(_v, v);
    ed25519_reduce(u);
    ed25519_reduce(v);

    for (int i = 0; i < 5; i++) r[i] = 0;
    if (u == 0)
    {
        r[0] = 1;
        return 0;
    }
    if (v == 0)
    {
        return 0;
    }

    mpz_class w;
    if (ed25519_div(w, u, v) != 0)
    {
        return 0;
    }
    mpz_class root;
    bool is_square = ed25519_sqrt(w, root);
    if (!is_square)
    {
        mpz_class w2 = w * 2;
        ed25519_reduce(w2);
        if (!ed25519_sqrt(w2, root))
        {
            printf("Ed25519FpSqrtRatio() 2 is not a non-residue mod p\n");
            return -1;
        }
    }
    // Return the even root
    if (mpz_odd_p(root.get_mpz_t()))
    {
        root = ED25519_P - root;
    }
    r[0] = is_square ? 1 : 0;
    scalar2array(root, &r[1]);
    return 0;
}

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
