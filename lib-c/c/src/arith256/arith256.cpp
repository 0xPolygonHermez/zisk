#include "arith256.hpp"
#include "../common/utils.hpp"

int Arith256 (
    const uint64_t * _a,  // 4 x 64 bits
    const uint64_t * _b,  // 4 x 64 bits
    const uint64_t * _c,  // 4 x 64 bits
          uint64_t * _dl, // 4 x 64 bits
          uint64_t * _dh  // 4 x 64 bits
)
{
    // Convert input parameters to scalars
    mpz_class a, b, c;
    array2scalar(_a, a);
    array2scalar(_b, b);
    array2scalar(_c, c);

    // Calculate the result as a scalar
    mpz_class d;
    d = (a * b) + c;

    // Decompose d = dl + dh<<256 (dh = d)
    mpz_class dl;
    dl = d & ScalarMask256;
    d >>= 256;

    // Convert scalars to output parameters
    scalar2array(dl, _dl);
    scalar2array(d, _dh);

    return 0;
}

int Arith256Mod (
    const uint64_t * _a,      // 4 x 64 bits
    const uint64_t * _b,      // 4 x 64 bits
    const uint64_t * _c,      // 4 x 64 bits
    const uint64_t * _module, // 4 x 64 bits
          uint64_t * _d       // 4 x 64 bits
)
{
    // Convert input parameters to scalars
    mpz_class a, b, c, module;
    array2scalar(_a, a);
    array2scalar(_b, b);
    array2scalar(_c, c);
    array2scalar(_module, module);

    // Calculate the result as a scalar
    mpz_class d;
    d = ((a * b) + c) % module;

    // Convert scalar to output parameter
    scalar2array(d, _d);

    return 0;
}