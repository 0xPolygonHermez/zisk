#ifndef POSEIDON_GOLDILOCKS_AVX
#define POSEIDON_GOLDILOCKS_AVX

#include "poseidon_goldilocks.hpp"
#include "goldilocks_base_field.hpp"
#include <immintrin.h>

inline void PoseidonGoldilocks::hash(Goldilocks::Element (&state)[CAPACITY], Goldilocks::Element const (&input)[SPONGE_WIDTH])
{
    Goldilocks::Element aux[SPONGE_WIDTH];
    hash_full_result(aux, input);
    std::memcpy(state, aux, CAPACITY * sizeof(Goldilocks::Element));
}

inline void PoseidonGoldilocks::pow7_avx(__m256i &st0, __m256i &st1, __m256i &st2)
{
    __m256i pw2_0, pw2_1, pw2_2;
    Goldilocks::square_avx(pw2_0, st0);
    Goldilocks::square_avx(pw2_1, st1);
    Goldilocks::square_avx(pw2_2, st2);
    __m256i pw4_0, pw4_1, pw4_2;
    Goldilocks::square_avx(pw4_0, pw2_0);
    Goldilocks::square_avx(pw4_1, pw2_1);
    Goldilocks::square_avx(pw4_2, pw2_2);
    __m256i pw3_0, pw3_1, pw3_2;
    Goldilocks::mult_avx(pw3_0, pw2_0, st0);
    Goldilocks::mult_avx(pw3_1, pw2_1, st1);
    Goldilocks::mult_avx(pw3_2, pw2_2, st2);

    Goldilocks::mult_avx(st0, pw3_0, pw4_0);
    Goldilocks::mult_avx(st1, pw3_1, pw4_1);
    Goldilocks::mult_avx(st2, pw3_2, pw4_2);
};

inline void PoseidonGoldilocks::add_avx(__m256i &st0, __m256i &st1, __m256i &st2, const Goldilocks::Element C_[SPONGE_WIDTH])
{
    __m256i c0, c1, c2;
    Goldilocks::load_avx(c0, &(C_[0]));
    Goldilocks::load_avx(c1, &(C_[4]));
    Goldilocks::load_avx(c2, &(C_[8]));
    Goldilocks::add_avx(st0, st0, c0);
    Goldilocks::add_avx(st1, st1, c1);
    Goldilocks::add_avx(st2, st2, c2);
}
// Assuming C_a is aligned
inline void PoseidonGoldilocks::add_avx_a(__m256i &st0, __m256i &st1, __m256i &st2, const Goldilocks::Element C_a[SPONGE_WIDTH])
{
    __m256i c0, c1, c2;
    Goldilocks::load_avx_a(c0, &(C_a[0]));
    Goldilocks::load_avx_a(c1, &(C_a[4]));
    Goldilocks::load_avx_a(c2, &(C_a[8]));
    Goldilocks::add_avx(st0, st0, c0);
    Goldilocks::add_avx(st1, st1, c1);
    Goldilocks::add_avx(st2, st2, c2);
}
inline void PoseidonGoldilocks::add_avx_small(__m256i &st0, __m256i &st1, __m256i &st2, const Goldilocks::Element C_small[SPONGE_WIDTH])
{
    __m256i c0, c1, c2;
    Goldilocks::load_avx(c0, &(C_small[0]));
    Goldilocks::load_avx(c1, &(C_small[4]));
    Goldilocks::load_avx(c2, &(C_small[8]));

    Goldilocks::add_avx_b_small(st0, st0, c0);
    Goldilocks::add_avx_b_small(st1, st1, c1);
    Goldilocks::add_avx_b_small(st2, st2, c2);
}

#endif