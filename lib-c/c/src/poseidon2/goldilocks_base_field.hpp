#ifndef GOLDILOCKS_BASE
#define GOLDILOCKS_BASE

#include <stdint.h> // uint64_t
#include <string>   // string
#include <gmpxx.h>
#include <iostream> // string
#include <omp.h>
#include <cassert>

#define GOLDILOCKS_DEBUG 0
#ifndef USE_ASSEMBLY
#define USE_ASSEMBLY 1  // Default value if not set by the Makefile
#endif
#define GOLDILOCKS_NUM_ROOTS 33
#define GOLDILOCKS_PRIME 0xFFFFFFFF00000001ULL
#define GOLDILOCKS_PRIME_NEG 0xFFFFFFFF
#define MSB_ 0x8000000000000000 // Most Significant Bit

class Goldilocks
{
public:
    typedef struct
    {
        uint64_t fe;
    } Element;

private:
    static const Element ZR;
    static const Element Q;
    static const Element MM;
    static const Element CQ;
    static const Element R2;
    static const Element TWO32;

    static const Element ZERO;
    static const Element ONE;
    static const Element NEGONE;
    static const Element SHIFT;
    static const Element W[GOLDILOCKS_NUM_ROOTS];

public:
    /*
        Basic functionality
    */

    static const Element &zero();
    static void zero(Element &result);

    static const Element &one();
    static void one(Element &result);

    static const Element &negone();
    static void negone(Element &result);

    static const Element &shift();
    static void shift(Element &result);

    static const Element &w(uint64_t i);
    static void w(Element &result, uint64_t i);

    static Element fromU64(uint64_t in1);
    static void fromU64(Element &result, uint64_t in1);
    static Element fromS64(int64_t in1);
    static void fromS64(Element &result, int64_t in1);
    static Element fromS32(int32_t in1);
    static void fromS32(Element &result, int32_t in1);
    static Element fromString(const std::string &in1, int radix = 10);
    static void fromString(Element &result, const std::string &in1, int radix = 10);
    static Element fromScalar(const mpz_class &scalar);
    static void fromScalar(Element &result, const mpz_class &scalar);

    static uint64_t toU64(const Element &in1);
    static void toU64(uint64_t &result, const Element &in1);
    static int64_t toS64(const Element &in1);
    static void toS64(int64_t &result, const Element &in1);
    static bool toS32(int32_t &result, const Element &in1);
    static std::string toString(const Element &in1, int radix = 10);
    static void toString(std::string &result, const Element &in1, int radix = 10);
    static std::string toString(const Element *in1, const uint64_t size, int radix = 10);

    /*
        Scalar operations
    */
    static void copy(Element &dst, const Element &src);
    static void copy(Element *dst, const Element *src);

    static void parcpy(Element *dst, const Element *src, uint64_t size, int num_threads_copy = 64);
    static void parSetZero(Element *dst, uint64_t size, int num_threads_copy = 64);

    static Element add(const Element &in1, const Element &in2);
    static void add(Element &result, const Element &in1, const Element &in2);
    static void add_no_double_carry(uint64_t &result, const uint64_t &in1, const uint64_t &in2);
    static Element inc(const Goldilocks::Element &fe);

    static Element sub(const Element &in1, const Element &in2);
    static void sub(Element &result, const Element &in1, const Element &in2);
    static Element dec(const Goldilocks::Element &fe);

    static Element mul(const Element &in1, const Element &in2);
    static void mul(Element &result, const Element &in1, const Element &in2);
    static void mul1(Element &result, const Element &in1, const Element &in2);
    static void mul2(Element &result, const Element &in1, const Element &in2);

    static Element square(const Element &in1);
    static void square(Element &result, const Element &in1);

    static Element pow(const Element& base, uint64_t exp);

    static Element div(const Element &in1, const Element &in2);
    static void div(Element &result, const Element &in1, const Element &in2);

    static Element neg(const Element &in1);
    static void neg(Element &result, const Element &in1);

    static bool isZero(const Element &in1);
    static bool isOne(const Element &in1);
    static bool isNegone(const Element &in1);

    static bool equal(const Element &in1, const Element &in2);

    static Element inv(const Element &in1);
    static void inv(Element &result, const Element &in1);

    static Element mulScalar(const Element &base, const uint64_t &scalar);
    static void mulScalar(Element &result, const Element &base, const uint64_t &scalar);

    static Element exp(Element base, uint64_t exp);
    static void exp(Element &result, Element base, uint64_t exps);

    static void batchInverse(Element *res, const Element *src, uint64_t size)
    {
        Element* tmp = new Element[size];
        copy(tmp[0], src[0]);

        for (uint64_t i = 1; i < size; i++)
        {
            mul(tmp[i], tmp[i - 1], src[i]);
        }

        Element z, z2;
        inv(z, tmp[size - 1]);

        for (uint64_t i = size - 1; i > 0; i--)
        {
            mul(z2, z, src[i]);
            mul(res[i], z, tmp[i - 1]);
            copy(z, z2);
        }
        copy(res[0], z);

        delete[] tmp;
    }
};

/*
    Operator Overloading
*/
inline Goldilocks::Element operator+(const Goldilocks::Element &in1, const Goldilocks::Element &in2) { return Goldilocks::add(in1, in2); }
inline Goldilocks::Element operator*(const Goldilocks::Element &in1, const Goldilocks::Element &in2) { return Goldilocks::mul(in1, in2); }
inline Goldilocks::Element operator-(const Goldilocks::Element &in1, const Goldilocks::Element &in2) { return Goldilocks::sub(in1, in2); }
inline Goldilocks::Element operator/(const Goldilocks::Element &in1, const Goldilocks::Element &in2) { return Goldilocks::div(in1, in2); }
inline bool operator==(const Goldilocks::Element &in1, const Goldilocks::Element &in2) { return Goldilocks::equal(in1, in2); }
inline Goldilocks::Element operator-(const Goldilocks::Element &in1) { return Goldilocks::neg(in1); }
inline Goldilocks::Element operator+(const Goldilocks::Element &in1) { return in1; }

#include "goldilocks_base_field_tools.hpp"
#include "goldilocks_base_field_scalar.hpp"

#endif // GOLDILOCKS_BASE
