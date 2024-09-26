#ifndef GOLDILOCKS_CUBIC_EXTENSION_PACK
#define GOLDILOCKS_CUBIC_EXTENSION_PACK
#include "goldilocks_base_field.hpp"
#include "goldilocks_cubic_extension.hpp"
#include <cassert>
/*
    Implementations for expressions:
*/

inline void Goldilocks3::copy_pack( uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_)
{
    for(uint64_t irow =0; irow<nrowsPack; ++irow){
        Goldilocks::copy(c_[irow], a_[irow]);
        Goldilocks::copy(c_[nrowsPack + irow], a_[nrowsPack + irow]);
        Goldilocks::copy(c_[2*nrowsPack + irow], a_[2*nrowsPack + irow]);   
    }
}

inline void Goldilocks3::add_pack( uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_, const Goldilocks::Element *b_)
{
    for(uint64_t irow =0; irow<nrowsPack; ++irow){
        c_[irow] = a_[irow] + b_[irow];
        c_[nrowsPack + irow] = a_[nrowsPack + irow] + b_[nrowsPack + irow];
        c_[2*nrowsPack + irow] = a_[2*nrowsPack + irow] + b_[2*nrowsPack + irow];
    }
}

inline void Goldilocks3::sub_pack( uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_, const Goldilocks::Element *b_)
{
    for(uint64_t irow =0; irow<nrowsPack; ++irow){
        c_[irow] = a_[irow] - b_[irow];
        c_[nrowsPack + irow] = a_[nrowsPack + irow] - b_[nrowsPack + irow];
        c_[2*nrowsPack + irow] = a_[2*nrowsPack + irow] - b_[2*nrowsPack + irow];
    }
}

inline void Goldilocks3::mul_pack(uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_, const Goldilocks::Element *b_)
{
    for (uint64_t i = 0; i < nrowsPack; ++i)
    {
        Goldilocks::Element A = (a_[i] + a_[nrowsPack + i]) * (b_[i] + b_[nrowsPack + i]);
        Goldilocks::Element B = (a_[i] + a_[2*nrowsPack + i]) * (b_[i] + b_[2*nrowsPack + i]);
        Goldilocks::Element C = (a_[nrowsPack + i] + a_[2*nrowsPack + i]) * (b_[nrowsPack + i] + b_[2*nrowsPack + i]);
        Goldilocks::Element D = a_[i] * b_[i];
        Goldilocks::Element E = a_[nrowsPack + i] * b_[nrowsPack + i];
        Goldilocks::Element F = a_[2*nrowsPack + i] * b_[2*nrowsPack + i];
        Goldilocks::Element G = D - E;

        c_[i] = (C + G) - F;
        c_[nrowsPack + i] = ((((A + C) - E) - E) - D);
        c_[2*nrowsPack + i] = B - G;
    }
};

inline void Goldilocks3::mul_pack(uint64_t nrowsPack, Goldilocks::Element *c_, const Goldilocks::Element *a_, const Goldilocks::Element *challenge_, const Goldilocks::Element *challenge_ops_)
{   
    for (uint64_t i = 0; i < nrowsPack; ++i)
    {
        Goldilocks::Element A = (a_[i] + a_[nrowsPack + i]) * challenge_ops_[i];
        Goldilocks::Element B = (a_[i] + a_[2*nrowsPack + i]) * challenge_ops_[nrowsPack + i];
        Goldilocks::Element C = (a_[nrowsPack + i] + a_[2*nrowsPack + i]) * challenge_ops_[2*nrowsPack + i];
        Goldilocks::Element D = a_[i] * challenge_[i];
        Goldilocks::Element E = a_[nrowsPack + i] * challenge_[nrowsPack + i];
        Goldilocks::Element F = a_[2*nrowsPack + i] * challenge_[2*nrowsPack + i];
        Goldilocks::Element G = D - E;

        c_[i] = (C + G) - F;
        c_[nrowsPack + i] = ((((A + C) - E) - E) - D);
        c_[2*nrowsPack + i] = B - G;
    }
};

inline void Goldilocks3::op_pack( uint64_t nrowsPack, uint64_t op, Goldilocks::Element *c, const Goldilocks::Element *a, const Goldilocks::Element *b)
{
    switch (op)
    {
    case 0:
        add_pack(nrowsPack, c, a, b);
        break;
    case 1:
        sub_pack(nrowsPack, c, a, b);
        break;
    case 2:
        mul_pack(nrowsPack, c, a, b);
        break;
    case 3:
        sub_pack(nrowsPack, c, b, a);
        break;
    default:
        assert(0);
        break;
    }
}

inline void Goldilocks3::op_31_pack( uint64_t nrowsPack, uint64_t op, Goldilocks::Element *c, const Goldilocks::Element *a, const Goldilocks::Element *b)
{
    switch (op)
    {
    case 0:
        for (uint64_t i = 0; i < nrowsPack; ++i)
        {
            c[i] = a[i] + b[i];
            c[nrowsPack + i] = a[nrowsPack + i];
            c[2*nrowsPack + i] = a[2*nrowsPack + i];
        }
        break;
    case 1:
        for (uint64_t i = 0; i < nrowsPack; ++i)
        {
            c[i] = a[i] - b[i];
            c[nrowsPack + i] = a[nrowsPack + i];
            c[2*nrowsPack + i] = a[2*nrowsPack + i];
        }
        break;
    case 2:
        for (uint64_t i = 0; i < nrowsPack; ++i)
        {
            c[i] = a[i] * b[i];
            c[nrowsPack + i] = a[nrowsPack + i] * b[i];
            c[2*nrowsPack + i] = a[2*nrowsPack + i] * b[i];
        }
        break;
    case 3:
        for (uint64_t i = 0; i < nrowsPack; ++i)
        {
            c[i] = b[i] - a[i];
            c[nrowsPack + i] = -a[nrowsPack + i];
            c[2*nrowsPack + i] = -a[2*nrowsPack + i];
        }
        break;
    default:
        assert(0);
        break;
    }
}

#endif