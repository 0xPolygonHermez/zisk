#ifndef UTILS_HPP
#define UTILS_HPP

#include <gmpxx.h>
#include "../ffiasm/fec.hpp"
#include "../ffiasm/fnec.hpp"
#include "../ffiasm/fq.hpp"
#include "globals.hpp"

// Converts and array of 4 u64 LE to a scalar
inline void array2scalar (const uint64_t * a, mpz_class &s)
{
    mpz_import(s.get_mpz_t(), 4, -1, 8, -1, 0, (const void *)a);
}

// Converts a 256 bits scalar to an array of 4 u64 LE
inline void scalar2array (mpz_class &s, uint64_t * a)
{
    // Pre-set to zero in case the scalar is smaller than 256 bits
    a[0] = 0;
    a[1] = 0;
    a[2] = 0;
    a[3] = 0;
    mpz_export((void *)a, NULL, -1, 8, -1, 0, s.get_mpz_t());
}

// Converts an array of 4 u64 LE to a FEC element
inline void array2fe (const uint64_t * a, RawFec::Element &fe)
{
    mpz_class s;
    array2scalar(a, s);
    fec.fromMpz(fe, s.get_mpz_t());
}

// Converts a FEC element to an array of 4 u64 LE
inline void fe2array (const RawFec::Element &fe, uint64_t * a)
{
    mpz_class s;
    fec.toMpz(s.get_mpz_t(), fe);
    scalar2array(s, a);
}

// Converts an array of 4 u64 LE to a FNEC element
inline void array2fe (const uint64_t * a, RawFnec::Element &fe)
{
    mpz_class s;
    array2scalar(a, s);
    fnec.fromMpz(fe, s.get_mpz_t());
}

// Converts a FNEC element to an array of 4 u64 LE
inline void fe2array (const RawFnec::Element &fe, uint64_t * a)
{
    mpz_class s;
    fnec.toMpz(s.get_mpz_t(), fe);
    scalar2array(s, a);
}

// Converts an array of 4 u64 LE to a Fq (BN254) element
inline void array2fe (const uint64_t * a, RawFq::Element &fe)
{
    mpz_class s;
    array2scalar(a, s);
    bn254.fromMpz(fe, s.get_mpz_t());
}

// Converts a Fq (BN254) element to an array of 4 u64 LE
inline void fe2array (const RawFq::Element &fe, uint64_t * a)
{
    mpz_class s;
    bn254.toMpz(s.get_mpz_t(), fe);
    scalar2array(s, a);
}

#endif