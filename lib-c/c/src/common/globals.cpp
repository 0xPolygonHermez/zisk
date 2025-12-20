#include "globals.hpp"

RawFec fec;
RawFnec fnec;
RawFq bn254;
RawBLS12_381_384 bls12_381;

mpz_class ScalarMask256 ("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16);
mpz_class ScalarMask384 ("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16);
mpz_class ScalarP_DIV_4 ("680447a8e5ff9a692c6e9ed90d2eb35d91dd2e13ce144afd9cc34a83dac3d8907aaffffac54ffffee7fbfffffffeaab", 16);
mpz_class ScalarP ("1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab", 16);
mpz_class ScalarNQR ("2", 16); // First non-quadratic residue in Fp