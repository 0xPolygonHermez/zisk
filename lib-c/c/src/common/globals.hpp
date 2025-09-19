#ifndef GLOBALS_HPP
#define GLOBALS_HPP

#include <gmpxx.h>
#include "../ffiasm/fec.hpp"
#include "../ffiasm/fnec.hpp"
#include "../ffiasm/fq.hpp"
#include "../ffiasm/bls12_381_384.hpp"

extern RawFec fec;
extern RawFnec fnec;
extern RawFq bn254;
extern RawBLS12_381_384 bls12_381;

extern mpz_class ScalarMask256;
extern mpz_class ScalarMask384;

#endif