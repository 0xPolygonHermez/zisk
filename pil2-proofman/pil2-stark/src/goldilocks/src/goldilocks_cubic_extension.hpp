#ifndef GOLDILOCKS_F3
#define GOLDILOCKS_F3

#include <stdint.h> // for uint64_t
#include "goldilocks_base_field.hpp"
#include <immintrin.h>
#include <cassert>
#include <vector>

#define FIELD_EXTENSION 3

/*
This is a field extension 3 of the goldilocks:
Prime: 0xFFFFFFFF00000001
Irreducible polynomial: x^3 - x -1
*/

class Goldilocks3
{
public:
    typedef Goldilocks::Element Element[FIELD_EXTENSION];
    typedef __m256i Element_avx[FIELD_EXTENSION];
    typedef __m512i Element_avx512[FIELD_EXTENSION];

private:
    static const Element ZERO;
    static const Element ONE;
    static const Element NEGONE;

public:
    uint64_t m = 1 * FIELD_EXTENSION;
    uint64_t p = GOLDILOCKS_PRIME;
    uint64_t n64 = 1;
    uint64_t n32 = n64 * 2;
    uint64_t n8 = n32 * 4;

    static inline const Element &zero() { return ZERO; };
    static inline void zero(Element &result)
    {
        result[0] = Goldilocks::zero();
        result[1] = Goldilocks::zero();
        result[2] = Goldilocks::zero();
    };
    static inline const Element &one() { return ONE; };
    static inline void one(Element &result)
    {
        result[0] = Goldilocks::one();
        result[1] = Goldilocks::zero();
        result[2] = Goldilocks::zero();
    };
    static inline bool isOne(Element &result)
    {
        return Goldilocks::isOne(result[0]) && Goldilocks::isOne(result[0]) && Goldilocks::isOne(result[0]);
    };
    static inline void fromU64(Element &result, uint64_t in1[FIELD_EXTENSION])
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            result[i] = Goldilocks::fromU64(in1[i]);
        }
    }
    static inline void fromS32(Element &result, int32_t in1[FIELD_EXTENSION])
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            result[i] = Goldilocks::fromS32(in1[i]);
        }
    }
    static inline void toU64(uint64_t (&result)[FIELD_EXTENSION], const Element &in1)
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            result[i] = Goldilocks::toU64(in1[i]);
        }
    }
    static inline std::vector<Goldilocks::Element> toVector(const Element &in1)
    {
        std::vector<Goldilocks::Element> result;
        result.assign(in1, in1 + FIELD_EXTENSION);
        return result;
    }
    static inline std::vector<Goldilocks::Element> toVector(const Element *in1)
    {

        std::vector<Goldilocks::Element> result;
        result.assign(*in1, *in1 + FIELD_EXTENSION);
        return result;
    }
    static inline std::string toString(const Element &in1, int radix = 10)
    {
        std::string res;
        toString(res, in1, radix);
        return res;
    }
    static inline void toString(std::string &result, const Element &in1, int radix = 10)
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            result += Goldilocks::toString(in1[i]);
            (i != FIELD_EXTENSION - 1) ? result += " , " : "";
        }
    }
    static inline void toString(std::string (&result)[FIELD_EXTENSION], const Element &in1, int radix = 10)
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            result[i] = Goldilocks::toString(in1[i]);
        }
    }
    static std::string toString(const Element *in1, const uint64_t size, int radix)
    {
        std::string result = "";
        for (uint64_t i = 0; i < size; i++)
        {
            result += toString(in1[i], 10);
            result += "\n";
        }
        return result;
    }
    static inline void fromString(Element &result, const std::string (&in1)[FIELD_EXTENSION], int radix = 10)
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            result[i] = Goldilocks::fromString(in1[i]);
        }
    }
    
    // ======== COPY =======

    static inline void copy(Element &dst, const Element &src)
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            Goldilocks::copy(dst[i], src[i]);
        }
    };
    static inline void copy(Element *dst, const Element *src)
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            Goldilocks::copy((*dst)[i], (*src)[i]);
        }
    };

    // ======== ADD ========
    static inline void add(Element &result, const Element &a, const uint64_t &b)
    {
        result[0] = a[0] + Goldilocks::fromU64(b);
        result[1] = a[1];
        result[2] = a[2];
    }
    static inline void add(Element &result, const Element &a, const Goldilocks::Element b)
    {
        result[0] = a[0] + b;
        result[1] = a[1];
        result[2] = a[2];
    }
    static inline void add(Element &result, const Goldilocks::Element a, const Element &b)
    {
        add(result, b, a);
    }
    static inline void add(Element &result, const Element &a, const Element &b)
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            result[i] = a[i] + b[i];
        }
    }

    // ======== SUB ========
    static inline void sub(Element &result, const Element &a, const uint64_t &b)
    {
        result[0] = a[0] - Goldilocks::fromU64(b);
        result[1] = a[1];
        result[2] = a[2];
    }
    static inline void sub(Element &result, const Goldilocks::Element a, const Element &b)
    {
        result[0] = a - b[0];
        result[1] = Goldilocks::neg(b[1]);
        result[2] = Goldilocks::neg(b[2]);
    }
    static inline void sub(Element &result, const Element &a, const Goldilocks::Element b)
    {
        result[0] = a[0] - b;
        result[1] = a[1];
        result[2] = a[2];
    }
    static inline void sub(Element &result, const Element &a, const Element &b)
    {
        for (uint64_t i = 0; i < FIELD_EXTENSION; i++)
        {
            result[i] = a[i] - b[i];
        }
    }
    
    // ======== NEG ========
    static inline void neg(Element &result, const Element &a)
    {
        sub(result, (Element &)zero(), a);
    }

    // ======== MUL ========
    static inline void mul(Element *result, const Element *a, const Element *b)
    {
        mul(*result, *a, *b);
    }
    static inline void mul(Element &result, const Element &a, const Element &b)
    {
        Goldilocks::Element A = (a[0] + a[1]) * (b[0] + b[1]);
        Goldilocks::Element B = (a[0] + a[2]) * (b[0] + b[2]);
        Goldilocks::Element C = (a[1] + a[2]) * (b[1] + b[2]);
        Goldilocks::Element D = a[0] * b[0];
        Goldilocks::Element E = a[1] * b[1];
        Goldilocks::Element F = a[2] * b[2];
        Goldilocks::Element G = D - E;

        result[0] = (C + G) - F;
        result[1] = ((((A + C) - E) - E) - D);
        result[2] = B - G;
    };
    static inline void mul(Element &result, const Element &a, const Goldilocks::Element &b)
    {
        result[0] = a[0] * b;
        result[1] = a[1] * b;
        result[2] = a[2] * b;
    }
    static inline void mul(Element &result, const Goldilocks::Element a, const Element &b)
    {
        mul(result, b, a);
    }
    static inline void mul(Element &result, const Element &a, const uint64_t b)
    {
        result[0] = a[0] * Goldilocks::fromU64(b);
        result[1] = a[1] * Goldilocks::fromU64(b);
        result[2] = a[2] * Goldilocks::fromU64(b);
    }

    // ======== DIV ========
    static inline void div(Element &result, const Element &a, const Goldilocks::Element b)
    {
        Goldilocks::Element b_inv = Goldilocks::inv(b);
        mul(result, a, b_inv);
    }

    // ======== MULSCALAR ========
    static inline void mulScalar(Element &result, const Element &a, const std::string &b)
    {
        result[0] = a[0] * Goldilocks::fromString(b);
        result[1] = a[1] * Goldilocks::fromString(b);
        result[2] = a[2] * Goldilocks::fromString(b);
    }

    // ======== SQUARE ========
    static inline void square(Element &result, const Element &a)
    {
        mul(result, a, a);
    }

    // ======== INV ========
    static inline void inv(Element *result, const Element *a)
    {
        inv(*result, *a);
    }
    static inline void inv(Element &result, const Element &a)
    {
        Goldilocks::Element aa = a[0] * a[0];
        Goldilocks::Element ac = a[0] * a[2];
        Goldilocks::Element ba = a[1] * a[0];
        Goldilocks::Element bb = a[1] * a[1];
        Goldilocks::Element bc = a[1] * a[2];
        Goldilocks::Element cc = a[2] * a[2];

        Goldilocks::Element aaa = aa * a[0];
        Goldilocks::Element aac = aa * a[2];
        Goldilocks::Element abc = ba * a[2];
        Goldilocks::Element abb = ba * a[1];
        Goldilocks::Element acc = ac * a[2];
        Goldilocks::Element bbb = bb * a[1];
        Goldilocks::Element bcc = bc * a[2];
        Goldilocks::Element ccc = cc * a[2];

        Goldilocks::Element t = abc + abc + abc + abb - aaa - aac - aac - acc - bbb + bcc - ccc;

        Goldilocks::Element tinv = Goldilocks::inv(t);
        Goldilocks::Element i1 = (bc + bb - aa - ac - ac - cc) * tinv;

        Goldilocks::Element i2 = (ba - cc) * tinv;
        Goldilocks::Element i3 = (ac + cc - bb) * tinv;

        result[0] = i1;
        result[1] = i2;
        result[2] = i3;
    }

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

    // ======== OPERATIONS ========

    /* Pack operations */

    static void copy_pack( uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_);
    static void add_pack( uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_, const Goldilocks::Element *b_);
    static void sub_pack( uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_, const Goldilocks::Element *b_);
    static void mul_pack(uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_, const Goldilocks::Element *b_);
    static void mul_pack(uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_, const Goldilocks::Element *challenge_, const Goldilocks::Element *challenge_ops_);
    
    static void op_pack( uint64_t nrowsPack, uint64_t op, Goldilocks::Element *c, const Goldilocks::Element *a, const Goldilocks::Element *b);
    static void op_31_pack( uint64_t nrowsPack, uint64_t op, Goldilocks::Element *c, const Goldilocks::Element *a, const Goldilocks::Element *b);

    /* AVX operations */
    static void copy_avx(Element_avx c_, const Element_avx a_);
    static void add_avx(Element_avx c_, const Element_avx a_, const Element_avx b_);
    static void sub_avx(Element_avx &c_, const Element_avx a_, const Element_avx b_);
    static void mul_avx(Element_avx &c_, const Element_avx &a_, const Element_avx &b_);
    static void mul_avx(Element_avx &c_, const Element_avx &a_, const Element_avx &challenge_, const Element_avx &challenge_ops_);
    
    static void op_avx(uint64_t op, Element_avx &c_, const Element_avx &a_, const Element_avx &b_);
    static void op_31_avx(uint64_t op, Element_avx &c_, const Element_avx &a_, const __m256i &b_);
    
#ifdef __AVX512__

    /* AVX512 operations */
    static void copy_avx512(Element_avx512 c_, Element_avx512 a_);
    static void add_avx512(Element_avx512 c_, const Element_avx512 a_, const Element_avx512 b_);
    static void sub_avx512(Element_avx512 &c_, const Element_avx512 a_, const Element_avx512 b_);
    static void mul_avx512(Element_avx512 &c_, const Element_avx512 &a_, const Element_avx512 &b_);
    static void mul_avx512(Element_avx512 &c_, const Element_avx512 &a_, const Element_avx512 &challenge_, const Element_avx512 &challenge_ops_);
    
    static void op_avx512(uint64_t op, Element_avx512 &c_, const Element_avx512 &a_, const Element_avx512 &b_);
    static void op_31_avx512(uint64_t op, Element_avx512 &c_, const Element_avx512 &a_, const __m512i &b_);

#endif
};

#endif // GOLDILOCKS_F3
