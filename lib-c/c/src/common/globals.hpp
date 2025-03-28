#ifndef GLOBALS_HPP
#define GLOBALS_HPP

#include <gmpxx.h>
#include "../ffiasm/fec.hpp"
#include "../ffiasm/fnec.hpp"

extern RawFec fec;
extern RawFnec fnec;

extern mpz_class ScalarMask256;

#endif