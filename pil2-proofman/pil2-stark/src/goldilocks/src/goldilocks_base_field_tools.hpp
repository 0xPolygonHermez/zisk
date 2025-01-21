#ifndef GOLDILOCKS_BASIC
#define GOLDILOCKS_BASIC
#include "goldilocks_base_field.hpp"


inline const Goldilocks::Element &Goldilocks::zero() { return ZERO; };
inline void Goldilocks::zero(Element &result) { result.fe = ZERO.fe; };

inline const Goldilocks::Element &Goldilocks::one() { return ONE; };
inline void Goldilocks::one(Element &result) { result.fe = ONE.fe; };

inline const Goldilocks::Element &Goldilocks::negone() { return NEGONE; };
inline void Goldilocks::negone(Element &result) { result.fe = NEGONE.fe; };

inline const Goldilocks::Element &Goldilocks::shift() { return SHIFT; };
inline void Goldilocks::shift(Element &result) { result.fe = SHIFT.fe; };

inline const Goldilocks::Element &Goldilocks::w(uint64_t i) { return W[i]; };
inline void Goldilocks::w(Element &result, uint64_t i) { result.fe = W[i].fe; };

inline Goldilocks::Element Goldilocks::fromU64(uint64_t in1)
{
    Goldilocks::Element res;
    Goldilocks::fromU64(res, in1);
    return res;
}

inline void Goldilocks::fromU64(Element &result, uint64_t in1)
{
    result.fe = in1;
}

inline Goldilocks::Element Goldilocks::fromS64(int64_t in1)
{
    Goldilocks::Element res;
    Goldilocks::fromS64(res, in1);
    return res;
}

inline void Goldilocks::fromS64(Element &result, int64_t in1)
{
    uint64_t aux;
    (in1 < 0) ? aux = static_cast<uint64_t>(in1) + GOLDILOCKS_PRIME : aux = static_cast<uint64_t>(in1);
    result.fe = aux;
}


inline Goldilocks::Element Goldilocks::fromS32(int32_t in1)
{
    Goldilocks::Element res;
    Goldilocks::fromS32(res, in1);
    return res;
}

inline void Goldilocks::fromS32(Element &result, int32_t in1)
{
    uint64_t aux;
    (in1 < 0) ? aux = static_cast<uint64_t>(in1) + GOLDILOCKS_PRIME : aux = static_cast<uint64_t>(in1);
    result.fe = aux;

}

inline Goldilocks::Element Goldilocks::fromString(const std::string &in1, int radix)
{
    Goldilocks::Element result;
    Goldilocks::fromString(result, in1, radix);
    return result;
};

inline void Goldilocks::fromString(Element &result, const std::string &in1, int radix)
{
    mpz_class aux(in1, radix);
    aux = (aux + (uint64_t)GOLDILOCKS_PRIME) % (uint64_t)GOLDILOCKS_PRIME;
    result.fe = aux.get_ui();

};

inline Goldilocks::Element Goldilocks::fromScalar(const mpz_class &scalar)
{
    Goldilocks::Element result;
    Goldilocks::fromScalar(result, scalar);
    return result;
};

inline void Goldilocks::fromScalar(Element &result, const mpz_class &scalar)
{
    mpz_class aux = (scalar + (uint64_t)GOLDILOCKS_PRIME) % (uint64_t)GOLDILOCKS_PRIME;
    result.fe = aux.get_ui();
};

inline uint64_t Goldilocks::toU64(const Element &in1)
{
    uint64_t res;
    Goldilocks::toU64(res, in1);
    return res;
};
inline void Goldilocks::toU64(uint64_t &result, const Element &in1)
{
    result = in1.fe;
    if( result >= GOLDILOCKS_PRIME )
        result -= GOLDILOCKS_PRIME;
};

inline int64_t Goldilocks::toS64(const Element &in1)
{
    int64_t res;
    Goldilocks::toS64(res, in1);
    return res;
}

/* Converts a field element into a signed 64bits integer */
inline void Goldilocks::toS64(int64_t &result, const Element &in1)
{
    mpz_class out = Goldilocks::toU64(in1);

    mpz_class maxInt(((uint64_t)GOLDILOCKS_PRIME - 1) / 2);

    if (out > maxInt)
    {
        mpz_class onegative = (uint64_t)GOLDILOCKS_PRIME - out;
        result = -onegative.get_si();
    }
    else
    {
        result = out.get_si();
    }
}

/* Converts a field element into a signed 32bits integer */
/* Precondition:  Goldilocks::Element < 2^31 */
inline bool Goldilocks::toS32(int32_t &result, const Element &in1)
{
    mpz_class out = Goldilocks::toU64(in1);

    mpz_class maxInt(0x7FFFFFFF);
    mpz_class minInt = (uint64_t)GOLDILOCKS_PRIME - 0x80000000;

    if (out > maxInt)
    {
        mpz_class onegative = (uint64_t)GOLDILOCKS_PRIME - out;
        if (out > minInt)
        {
            result = -onegative.get_si();
        }
        else
        {
            std::cerr << "Error: Goldilocks::toS32 accessing a non-32bit value: " << Goldilocks::toString(in1, 16) << " out=" << out.get_str(16) << " minInt=" << minInt.get_str(16) << " maxInt=" << maxInt.get_str(16) << std::endl;
            return false;
        }
    }
    else
    {
        result = out.get_si();
    }
    return true;
}

inline std::string Goldilocks::toString(const Element &in1, int radix)
{
    std::string result;
    Goldilocks::toString(result, in1, radix);
    return result;
}

inline void Goldilocks::toString(std::string &result, const Element &in1, int radix)
{
    mpz_class aux = Goldilocks::toU64(in1);
    result = aux.get_str(radix);
}

inline std::string Goldilocks::toString(const Element *in1, const uint64_t size, int radix)
{
    std::string result = "";
    for (uint64_t i = 0; i < size; i++)
    {
        mpz_class aux = Goldilocks::toU64(in1[i]);
        result += std::to_string(i) + ": " + aux.get_str(radix) + "\n";
    }
    return result;
}

#endif