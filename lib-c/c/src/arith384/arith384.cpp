#include "arith384.hpp"
#include "../common/utils.hpp"
#include "../common/mul_add.hpp"

int Arith384 (
    const uint64_t * _a,  // 6 x 64 bits
    const uint64_t * _b,  // 6 x 64 bits
    const uint64_t * _c,  // 6 x 64 bits
          uint64_t * _dl, // 6 x 64 bits
          uint64_t * _dh  // 6 x 64 bits
)
{
    // Convert input parameters to scalars
    mpz_class a, b, c;
    array2scalar6(_a, a);
    array2scalar6(_b, b);
    array2scalar6(_c, c);

    // Calculate the result as a scalar
    mpz_class d;
    d = (a * b) + c;

    // Decompose d = dl + dh<<256 (dh = d)
    mpz_class dl;
    dl = d & ScalarMask384;
    d >>= 384;

    // Convert scalars to output parameters
    scalar2array6(dl, _dl);
    scalar2array6(d, _dh);

    return 0;
}

int Arith384Mod (
    const uint64_t * _a,      // 6 x 64 bits
    const uint64_t * _b,      // 6 x 64 bits
    const uint64_t * _c,      // 6 x 64 bits
    const uint64_t * _module, // 6 x 64 bits
          uint64_t * _d       // 6 x 64 bits
)
{
    // Fast path for the BLS12-381 base field, the usual module: with a, b and c reduced to
    // [0, q), a * b = mont_mul(a, mont_mul(b, R^2)), since a Montgomery product of a plain value
    // and a Montgomery one is plain; then c is added with a modular addition
    if (memcmp(_module, BLS12_381_384_q.longVal, 6 * sizeof(uint64_t)) == 0)
    {
        RawBLS12_381_384::Element a, b, c, d;
        array2plain(_a, a);
        array2plain(_b, b);
        array2plain(_c, c);
        bls12_381.toMontgomery(b, b);
        bls12_381.mul(d, a, b);
        bls12_381.add(d, d, c);
        plain2array(d, _d);
        return 0;
    }

    // t = a * b + c, which fits in 12 limbs
    uint64_t t[12];
    mul_add<6>(t, _a, _b, _c);

    // d = t mod module, with GMP's low level division, which does not allocate; it needs the
    // most significant limb of the divisor to be non-zero
    mp_size_t dn = 6;
    while ((dn > 0) && (_module[dn - 1] == 0)) dn--;
    if (dn == 0)
    {
        printf("Arith384Mod() got module = 0\n");
        return -1;
    }
    mp_limb_t q[12], r[6] = {0, 0, 0, 0, 0, 0};
    mpn_tdiv_qr(q, r, 0, (const mp_limb_t *)t, 12, (const mp_limb_t *)_module, dn);

    memcpy(_d, r, 6 * sizeof(uint64_t));

    return 0;
}