#ifndef GLOBALS_HPP
#define GLOBALS_HPP

#include <gmpxx.h>
#include "../ffiasm/fec.hpp"
#include "../ffiasm/fnec.hpp"
#include "../ffiasm/fq.hpp"
#include "../ffiasm/bls12_381_384.hpp"
#include "../ffiasm/psecp256r1.hpp"
#include "../ffiasm/nsecp256r1.hpp"

extern RawFec fec;
extern RawFnec fnec;
extern RawFq bn254;
extern RawBLS12_381_384 bls12_381;
extern RawpSecp256r1 secp256r1;
extern RawnSecp256r1 secp256r1n;

extern mpz_class ScalarMask256;
extern mpz_class ScalarMask384;
extern mpz_class ScalarP_DIV_4;
extern mpz_class ScalarP;
extern mpz_class ScalarNQR_FP;
extern mpz_class ScalarP_MINUS_3_DIV_4;
extern mpz_class ScalarP_MINUS_1_DIV_2;

#endif