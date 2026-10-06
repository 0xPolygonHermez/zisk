#include <gmpxx.h>
#include <stdint.h>
#include "babyjubjub.hpp"
#include "../ffiasm/fr.hpp"
#include "../common/utils.hpp"

// BN254 scalar field (BabyJubJub base field) instance, local to this unit.
static RawFr babyjubjub_fr;

// Twisted-Edwards parameters (circomlib-compatible).
static const unsigned long int BABYJUBJUB_A = 168700;
static const unsigned long int BABYJUBJUB_D = 168696;

// Converts an array of 4 u64 LE to an Fr element
static inline void array2fr (const uint64_t * a, RawFr::Element &fe)
{
    mpz_class s;
    array2scalar(a, s);
    babyjubjub_fr.fromMpz(fe, s.get_mpz_t());
}

// Converts an Fr element to an array of 4 u64 LE
static inline void fr2array (const RawFr::Element &fe, uint64_t * a)
{
    mpz_class s;
    babyjubjub_fr.toMpz(s.get_mpz_t(), fe);
    scalar2array(s, a);
}

static inline int BabyJubJubAddFe (const RawFr::Element &x1, const RawFr::Element &y1,
                                   const RawFr::Element &x2, const RawFr::Element &y2,
                                   RawFr::Element &x3, RawFr::Element &y3)
{
    RawFr::Element a, d, one, x1x2, y1y2, x1y2, y1x2, tau, dtau, num, den;

    babyjubjub_fr.fromUI(a, BABYJUBJUB_A);
    babyjubjub_fr.fromUI(d, BABYJUBJUB_D);
    babyjubjub_fr.fromUI(one, 1);

    babyjubjub_fr.mul(x1x2, x1, x2);
    babyjubjub_fr.mul(y1y2, y1, y2);
    babyjubjub_fr.mul(x1y2, x1, y2);
    babyjubjub_fr.mul(y1x2, y1, x2);

    // tau = x1*x2*y1*y2 ; dtau = d*tau
    babyjubjub_fr.mul(tau, x1x2, y1y2);
    babyjubjub_fr.mul(dtau, d, tau);

    // x3 = (x1*y2 + y1*x2) / (1 + d*tau)
    babyjubjub_fr.add(num, x1y2, y1x2);
    babyjubjub_fr.add(den, one, dtau);
    if (babyjubjub_fr.isZero(den))
    {
        printf("BabyJubJubAddFe() got denominator=0 (1 + d*tau)\n");
        return -1;
    }
    babyjubjub_fr.div(x3, num, den);

    // y3 = (y1*y2 - a*x1*x2) / (1 - d*tau)
    babyjubjub_fr.mul(num, a, x1x2);
    babyjubjub_fr.sub(num, y1y2, num);
    babyjubjub_fr.sub(den, one, dtau);
    if (babyjubjub_fr.isZero(den))
    {
        printf("BabyJubJubAddFe() got denominator=0 (1 - d*tau)\n");
        return -1;
    }
    babyjubjub_fr.div(y3, num, den);

    return 0;
}

#ifdef __cplusplus
extern "C" {
#endif

int BabyJubJubAdd (const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    RawFr::Element x1, y1, x2, y2, x3, y3;
    array2fr(_x1, x1);
    array2fr(_y1, y1);
    array2fr(_x2, x2);
    array2fr(_y2, y2);

    int result = BabyJubJubAddFe(x1, y1, x2, y2, x3, y3);

    fr2array(x3, _x3);
    fr2array(y3, _y3);

    return result;
}

int BabyJubJubAddP (const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    RawFr::Element x1, y1, x2, y2, x3, y3;
    array2fr(p1, x1);
    array2fr(p1 + 4, y1);
    array2fr(p2, x2);
    array2fr(p2 + 4, y2);

    int result = BabyJubJubAddFe(x1, y1, x2, y2, x3, y3);

    fr2array(x3, p3);
    fr2array(y3, p3 + 4);

    return result;
}

#ifdef __cplusplus
} // extern "C"
#endif
