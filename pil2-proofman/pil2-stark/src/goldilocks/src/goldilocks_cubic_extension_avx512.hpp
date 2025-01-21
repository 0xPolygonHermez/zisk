#ifndef GOLDILOCKS_CUBIC_EXTENSION_AVX512
#define GOLDILOCKS_CUBIC_EXTENSION_AVX512
#ifdef __AVX512__
#include "goldilocks_base_field.hpp"
#include "goldilocks_cubic_extension.hpp"
#include <cassert>
/*
    Implementations for expressions:
*/

inline void Goldilocks3::copy_avx512(Element_avx512 c_, Element_avx512 a_)
{
    c_[0] = a_[0];
    c_[1] = a_[1];
    c_[2] = a_[2];
}

inline void Goldilocks3::add_avx512(Element_avx512 c_, const Element_avx512 a_, const Element_avx512 b_)
{
    Goldilocks::add_avx512(c_[0], a_[0], b_[0]);
    Goldilocks::add_avx512(c_[1], a_[1], b_[1]);
    Goldilocks::add_avx512(c_[2], a_[2], b_[2]);
}

inline void Goldilocks3::sub_avx512(Element_avx512 &c_, const Element_avx512 a_, const Element_avx512 b_)
{
    Goldilocks::sub_avx512(c_[0], a_[0], b_[0]);
    Goldilocks::sub_avx512(c_[1], a_[1], b_[1]);
    Goldilocks::sub_avx512(c_[2], a_[2], b_[2]);
}

inline void Goldilocks3::mul_avx512(Element_avx512 &c_, const Element_avx512 &a_, const Element_avx512 &b_)
{

    __m512i aux0_, aux1_, aux2_;
    __m512i A_, B_, C_, D_, E_, F_, G_;
    __m512i auxr_;

    Goldilocks::add_avx512(A_, a_[0], a_[1]);
    Goldilocks::add_avx512(B_, a_[0], a_[2]);
    Goldilocks::add_avx512(C_, a_[1], a_[2]);
    Goldilocks::add_avx512(aux0_, b_[0], b_[1]);
    Goldilocks::add_avx512(aux1_, b_[0], b_[2]);
    Goldilocks::add_avx512(aux2_, b_[1], b_[2]);
    Goldilocks::mult_avx512(A_, A_, aux0_);
    Goldilocks::mult_avx512(B_, B_, aux1_);
    Goldilocks::mult_avx512(C_, C_, aux2_);
    Goldilocks::mult_avx512(D_, a_[0], b_[0]);
    Goldilocks::mult_avx512(E_, a_[1], b_[1]);
    Goldilocks::mult_avx512(F_, a_[2], b_[2]);
    Goldilocks::sub_avx512(G_, D_, E_);

    Goldilocks::add_avx512(c_[0], C_, G_);
    Goldilocks::sub_avx512(c_[0], c_[0], F_);
    Goldilocks::add_avx512(c_[1], A_, C_);
    Goldilocks::add_avx512(auxr_, E_, E_);
    Goldilocks::add_avx512(auxr_, auxr_, D_);
    Goldilocks::sub_avx512(c_[1], c_[1], auxr_);
    Goldilocks::sub_avx512(c_[2], B_, G_);
};

inline void Goldilocks3::mul_avx512(Element_avx512 &c_, const Element_avx512 &a_, const Element_avx512 &challenge_, const Element_avx512 &challenge_ops_)
{
    __m512i A_, B_, C_, D_, E_, F_, G_;
    __m512i auxr_;

    Goldilocks::add_avx512(A_, a_[0], a_[1]);
    Goldilocks::add_avx512(B_, a_[0], a_[2]);
    Goldilocks::add_avx512(C_, a_[1], a_[2]);
    Goldilocks::mult_avx512(A_, A_, challenge_ops_[0]);
    Goldilocks::mult_avx512(B_, B_, challenge_ops_[1]);
    Goldilocks::mult_avx512(C_, C_, challenge_ops_[2]);
    Goldilocks::mult_avx512(D_, a_[0], challenge_[0]);
    Goldilocks::mult_avx512(E_, a_[1], challenge_[1]);
    Goldilocks::mult_avx512(F_, a_[2], challenge_[2]);
    Goldilocks::sub_avx512(G_, D_, E_);

    Goldilocks::add_avx512(c_[0], C_, G_);
    Goldilocks::sub_avx512(c_[0], c_[0], F_);
    Goldilocks::add_avx512(c_[1], A_, C_);
    Goldilocks::add_avx512(auxr_, E_, E_);
    Goldilocks::add_avx512(auxr_, auxr_, D_);
    Goldilocks::sub_avx512(c_[1], c_[1], auxr_);
    Goldilocks::sub_avx512(c_[2], B_, G_);
};

inline void Goldilocks3::op_avx512(uint64_t op, Element_avx512 &c_, const Element_avx512 &a_, const Element_avx512 &b_)
{
    switch (op)
    {
    case 0:
        add_avx512(c_, a_, b_);
        break;
    case 1:
        sub_avx512(c_, a_, b_);
        break;
    case 2:
        mul_avx512(c_, a_, b_);
        break;
    case 3:
        sub_avx512(c_, b_, a_);
        break;
    default:
        assert(0);
        break;
    }
}

    
inline void Goldilocks3::op_31_avx512(uint64_t op, Element_avx512 &c_, const Element_avx512 &a_, const __m512i &b_)
{
    switch (op)
    {
    case 0:
        Goldilocks::add_avx512(c_[0], a_[0], b_);
        c_[1] = a_[1];
        c_[2] = a_[2];
        break;
    case 1:
        Goldilocks::sub_avx512(c_[0], a_[0], b_);
        c_[1] = a_[1];
        c_[2] = a_[2];
        break;
    case 2:
        Goldilocks::mult_avx512(c_[0], b_, a_[0]);
        Goldilocks::mult_avx512(c_[1], b_, a_[1]);
        Goldilocks::mult_avx512(c_[2], b_, a_[2]);
        break;
    case 3:
        Goldilocks::sub_avx512(c_[0], b_, a_[0]);
        Goldilocks::sub_avx512(c_[1], P8, a_[1]);
        Goldilocks::sub_avx512(c_[2], P8, a_[2]);
        break;
    default:
        assert(0);
        break;
    }
}
#endif
#endif