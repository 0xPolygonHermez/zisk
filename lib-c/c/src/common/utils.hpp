#ifndef UTILS_HPP
#define UTILS_HPP

#include <string.h>
#include <gmpxx.h>
#include "../ffiasm/fec.hpp"
#include "../ffiasm/fnec.hpp"
#include "../ffiasm/fq.hpp"
#include "globals.hpp"
#include "limbs.hpp"

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

// Converts and array of 6 u64 LE to a scalar
inline void array2scalar6 (const uint64_t * a, mpz_class &s)
{
    mpz_import(s.get_mpz_t(), 6, -1, 8, -1, 0, (const void *)a);
}

// Converts a 384 bits scalar to an array of 6 u64 LE
inline void scalar2array6 (mpz_class &s, uint64_t * a)
{
    // Pre-set to zero in case the scalar is smaller than 384 bits
    a[0] = 0;
    a[1] = 0;
    a[2] = 0;
    a[3] = 0;
    a[4] = 0;
    a[5] = 0;
    mpz_export((void *)a, NULL, -1, 8, -1, 0, s.get_mpz_t());
}

// Converts an array of u64 LE to a field element, in Montgomery form. This is what the field
// fromMpz() does, a limb copy plus a Montgomery conversion, without the GMP round trip
template <typename Field>
inline void array2fe_raw (Field &field, const uint64_t * a, typename Field::Element &fe)
{
    memcpy(fe.v, a, sizeof(fe.v));
    field.toMontgomery(fe, fe);
}

// Converts a field element, in Montgomery form, to an array of u64 LE. This is what the field
// toMpz() does, a Montgomery conversion plus a limb copy, without the GMP round trip
template <typename Field>
inline void fe2array_raw (Field &field, const typename Field::Element &fe, uint64_t * a)
{
    typename Field::Element tmp;
    field.fromMontgomery(tmp, fe);
    memcpy(a, tmp.v, sizeof(tmp.v));
}

// Converts an array of 4 u64 LE to a FEC element
inline void array2fe (const uint64_t * a, RawFec::Element &fe)
{
    array2fe_raw(fec, a, fe);
}

// Converts a FEC element to an array of 4 u64 LE
inline void fe2array (const RawFec::Element &fe, uint64_t * a)
{
    fe2array_raw(fec, fe, a);
}

// Converts an array of 4 u64 LE to a FNEC element
inline void array2fe (const uint64_t * a, RawFnec::Element &fe)
{
    array2fe_raw(fnec, a, fe);
}

// Converts a FNEC element to an array of 4 u64 LE
inline void fe2array (const RawFnec::Element &fe, uint64_t * a)
{
    fe2array_raw(fnec, fe, a);
}

// Converts an array of 4 u64 LE to a Fq (BN254) element
inline void array2fe (const uint64_t * a, RawFq::Element &fe)
{
    array2fe_raw(bn254, a, fe);
}

// Converts a Fq (BN254) element to an array of 4 u64 LE
inline void fe2array (const RawFq::Element &fe, uint64_t * a)
{
    fe2array_raw(bn254, fe, a);
}

// Converts an array of 6 u64 LE to a Fq (BLS12_381) element
inline void array2fe (const uint64_t * a, RawBLS12_381_384::Element &fe)
{
    array2fe_raw(bls12_381, a, fe);
}

// Converts a Fq (BLS12_381) element to an array of 6 u64 LE
inline void fe2array (const RawBLS12_381_384::Element &fe, uint64_t * a)
{
    fe2array_raw(bls12_381, fe, a);
}

// Converts an array of 4 u64 LE to a Fq (Secp256r1) element
inline void array2fe (const uint64_t * a, RawpSecp256r1::Element &fe)
{
    array2fe_raw(secp256r1, a, fe);
}

// Converts a Fq (Secp256r1) element to an array of 4 u64 LE
inline void fe2array (const RawpSecp256r1::Element &fe, uint64_t * a)
{
    fe2array_raw(secp256r1, fe, a);
}

// Converts an array of 4 u64 LE to a Fq (nSecp256r1) element
inline void array2fe (const uint64_t * a, RawnSecp256r1::Element &fe)
{
    array2fe_raw(secp256r1n, a, fe);
}

// Converts a Fq (nSecp256r1) element to an array of 4 u64 LE
inline void fe2array (const RawnSecp256r1::Element &fe, uint64_t * a)
{
    fe2array_raw(secp256r1n, fe, a);
}

// Plain (not Montgomery) field elements. Field additions and subtractions give the same result
// on plain values as on Montgomery ones, and a Montgomery multiplication of a plain value by a
// Montgomery one gives the plain product, so some operations can skip the conversions

// Converts an array of 4 u64 LE to a plain Fq (BN254) element, reduced to [0, q)
inline void array2plain (const uint64_t * a, RawFq::Element &fe)
{
    memcpy(fe.v, a, sizeof(fe.v));
    limbs_reduce<4>(fe.v, Fq_q.longVal);
}

// Converts a plain Fq (BN254) element to an array of 4 u64 LE
inline void plain2array (const RawFq::Element &fe, uint64_t * a)
{
    memcpy(a, fe.v, sizeof(fe.v));
}

// Converts an array of 6 u64 LE to a plain Fq (BLS12_381) element, reduced to [0, q)
inline void array2plain (const uint64_t * a, RawBLS12_381_384::Element &fe)
{
    memcpy(fe.v, a, sizeof(fe.v));
    limbs_reduce<6>(fe.v, BLS12_381_384_q.longVal);
}

// Converts a plain Fq (BLS12_381) element to an array of 6 u64 LE
inline void plain2array (const RawBLS12_381_384::Element &fe, uint64_t * a)
{
    memcpy(a, fe.v, sizeof(fe.v));
}

#endif