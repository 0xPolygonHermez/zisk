#include "arith384.hpp"
#include "../common/utils.hpp"

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
    // Convert input parameters to scalars
    mpz_class a, b, c, module;
    array2scalar6(_a, a);
    array2scalar6(_b, b);
    array2scalar6(_c, c);
    array2scalar6(_module, module);

    // Calculate the result as a scalar
    mpz_class d;
    d = ((a * b) + c) % module;

    // Convert scalar to output parameter
    scalar2array6(d, _d);

    return 0;
}