#ifndef GOLDILOCKS_CUBIC_EXTENSION_AVX
#define GOLDILOCKS_CUBIC_EXTENSION_AVX
#include "goldilocks_base_field.hpp"
#include "goldilocks_cubic_extension.hpp"
#include <cassert>
/*
    Implementations for expressions:
*/

inline void Goldilocks3::copy_avx(Element_avx c_, const Element_avx a_)
{
    c_[0] = a_[0];
    c_[1] = a_[1];
    c_[2] = a_[2];
}

inline void Goldilocks3::add_avx(Element_avx c_, const Element_avx a_, const Element_avx b_)
{
    Goldilocks::add_avx(c_[0], a_[0], b_[0]);
    Goldilocks::add_avx(c_[1], a_[1], b_[1]);
    Goldilocks::add_avx(c_[2], a_[2], b_[2]);
}

inline void Goldilocks3::sub_avx(Element_avx &c_, const Element_avx a_, const Element_avx b_)
{
    Goldilocks::sub_avx(c_[0], a_[0], b_[0]);
    Goldilocks::sub_avx(c_[1], a_[1], b_[1]);
    Goldilocks::sub_avx(c_[2], a_[2], b_[2]);
}

inline void Goldilocks3::mul_avx(Element_avx &c_, const Element_avx &a_, const Element_avx &b_)
{

    __m256i aux0_, aux1_, aux2_;
    __m256i A_, B_, C_, D_, E_, F_, G_;
    __m256i auxr_;

    Goldilocks::add_avx(A_, a_[0], a_[1]);
    Goldilocks::add_avx(B_, a_[0], a_[2]);
    Goldilocks::add_avx(C_, a_[1], a_[2]);
    Goldilocks::add_avx(aux0_, b_[0], b_[1]);
    Goldilocks::add_avx(aux1_, b_[0], b_[2]);
    Goldilocks::add_avx(aux2_, b_[1], b_[2]);
    Goldilocks::mult_avx(A_, A_, aux0_);
    Goldilocks::mult_avx(B_, B_, aux1_);
    Goldilocks::mult_avx(C_, C_, aux2_);
    Goldilocks::mult_avx(D_, a_[0], b_[0]);
    Goldilocks::mult_avx(E_, a_[1], b_[1]);
    Goldilocks::mult_avx(F_, a_[2], b_[2]);
    Goldilocks::sub_avx(G_, D_, E_);

    Goldilocks::add_avx(c_[0], C_, G_);
    Goldilocks::sub_avx(c_[0], c_[0], F_);
    Goldilocks::add_avx(c_[1], A_, C_);
    Goldilocks::add_avx(auxr_, E_, E_);
    Goldilocks::add_avx(auxr_, auxr_, D_);
    Goldilocks::sub_avx(c_[1], c_[1], auxr_);
    Goldilocks::sub_avx(c_[2], B_, G_);
};
inline void Goldilocks3::mul_avx(Element_avx &c_, const Element_avx &a_, const Element_avx &challenge_, const Element_avx &challenge_ops_)
{
    __m256i A_, B_, C_, D_, E_, F_, G_;
    __m256i auxr_;

    Goldilocks::add_avx(A_, a_[0], a_[1]);
    Goldilocks::add_avx(B_, a_[0], a_[2]);
    Goldilocks::add_avx(C_, a_[1], a_[2]);
    Goldilocks::mult_avx(A_, A_, challenge_ops_[0]);
    Goldilocks::mult_avx(B_, B_, challenge_ops_[1]);
    Goldilocks::mult_avx(C_, C_, challenge_ops_[2]);
    Goldilocks::mult_avx(D_, a_[0], challenge_[0]);
    Goldilocks::mult_avx(E_, a_[1], challenge_[1]);
    Goldilocks::mult_avx(F_, a_[2], challenge_[2]);
    Goldilocks::sub_avx(G_, D_, E_);

    Goldilocks::add_avx(c_[0], C_, G_);
    Goldilocks::sub_avx(c_[0], c_[0], F_);
    Goldilocks::add_avx(c_[1], A_, C_);
    Goldilocks::add_avx(auxr_, E_, E_);
    Goldilocks::add_avx(auxr_, auxr_, D_);
    Goldilocks::sub_avx(c_[1], c_[1], auxr_);
    Goldilocks::sub_avx(c_[2], B_, G_);
};

inline void Goldilocks3::op_avx(uint64_t op, Element_avx &c_, const Element_avx &a_, const Element_avx &b_)
{
    switch (op)
    {
    case 0:
        add_avx(c_, a_, b_);
        break;
    case 1:
        sub_avx(c_, a_, b_);
        break;
    case 2:
        mul_avx(c_, a_, b_);
        break;
    case 3:
        sub_avx(c_, b_, a_);
        break;
    default:
        assert(0);
        break;
    }
}
    
inline void Goldilocks3::op_31_avx(uint64_t op, Element_avx &c_, const Element_avx &a_, const __m256i &b_)
{
    switch (op)
    {
    case 0:
        Goldilocks::add_avx(c_[0], a_[0], b_);
        c_[1] = a_[1];
        c_[2] = a_[2];
        break;
    case 1:
        Goldilocks::sub_avx(c_[0], a_[0], b_);
        c_[1] = a_[1];
        c_[2] = a_[2];
        break;
    case 2:
        Goldilocks::mult_avx(c_[0], b_, a_[0]);
        Goldilocks::mult_avx(c_[1], b_, a_[1]);
        Goldilocks::mult_avx(c_[2], b_, a_[2]);
        break;
    case 3:
        Goldilocks::sub_avx(c_[0], b_, a_[0]);
        Goldilocks::sub_avx(c_[1], P, a_[1]);
        Goldilocks::sub_avx(c_[2], P, a_[2]);
        break;
    default:
        assert(0);
        break;
    }
}
#endif