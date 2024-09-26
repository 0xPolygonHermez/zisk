#ifndef POSEIDON_GOLDILOCKS
#define POSEIDON_GOLDILOCKS

#include "poseidon_goldilocks_constants.hpp"
#include "goldilocks_base_field.hpp"
#include <immintrin.h>

#define RATE 8
#define CAPACITY 4
#define HASH_SIZE 4
#define SPONGE_WIDTH (RATE + CAPACITY)
#define HALF_N_FULL_ROUNDS 4
#define N_FULL_ROUNDS_TOTAL (2 * HALF_N_FULL_ROUNDS)
#define N_PARTIAL_ROUNDS 22
#define N_ROUNDS (N_FULL_ROUNDS_TOTAL + N_PARTIAL_ROUNDS)

class PoseidonGoldilocks
{

private:
    inline void static pow7(Goldilocks::Element &x);
    inline void static pow7_(Goldilocks::Element *x);
    inline void static add_(Goldilocks::Element *x, const Goldilocks::Element C[SPONGE_WIDTH]);
    inline void static pow7add_(Goldilocks::Element *x, const Goldilocks::Element C[SPONGE_WIDTH]);
    inline void static mvp_(Goldilocks::Element *state, const Goldilocks::Element mat[SPONGE_WIDTH][SPONGE_WIDTH]);
    inline Goldilocks::Element static dot_(Goldilocks::Element *x, const Goldilocks::Element C[SPONGE_WIDTH]);
    inline void static prod_(Goldilocks::Element *x, const Goldilocks::Element alpha, const Goldilocks::Element C[SPONGE_WIDTH]);

    inline void static add_avx(__m256i &st0, __m256i &st1, __m256i &st2, const Goldilocks::Element C[SPONGE_WIDTH]);
    inline void static pow7_avx(__m256i &st0, __m256i &st1, __m256i &st2);
    inline void static add_avx_a(__m256i &st0, __m256i &st1, __m256i &st2, const Goldilocks::Element C[SPONGE_WIDTH]);
    inline void static add_avx_small(__m256i &st0, __m256i &st1, __m256i &st2, const Goldilocks::Element C[SPONGE_WIDTH]);

#ifdef __AVX512__
    inline void static pow7_avx512(__m512i &st0, __m512i &st1, __m512i &st2);
    inline void static add_avx512(__m512i &st0, __m512i &st1, __m512i &st2, const Goldilocks::Element C[SPONGE_WIDTH]);
    inline void static add_avx512_a(__m512i &st0, __m512i &st1, __m512i &st2, const Goldilocks::Element C[SPONGE_WIDTH]);
    inline void static add_avx512_small(__m512i &st0, __m512i &st1, __m512i &st2, const Goldilocks::Element C[SPONGE_WIDTH]);
#endif

public:
    // Wrapper:
    void static merkletree(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, int nThreads = 0, uint64_t dim = 1);
    void static merkletree_batch(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, uint64_t batch_size, int nThreads = 0, uint64_t dim = 1);

    // Non-vectorized:
    void static hash_full_result_seq(Goldilocks::Element *, const Goldilocks::Element *);
    void static linear_hash_seq(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t size);
    void static merkletree_seq(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, int nThreads = 0, uint64_t dim = 1);
    void static hash_seq(Goldilocks::Element (&state)[CAPACITY], const Goldilocks::Element (&input)[SPONGE_WIDTH]);
    void static merkletree_batch_seq(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, uint64_t batch_size, int nThreads = 0, uint64_t dim = 1);

    // Vectorized AVX:
    // Note, the functions that do not have the _avx suffix are the default ones to
    // be used in the prover, they implement avx vectorixation though.
    void static hash_full_result(Goldilocks::Element *, const Goldilocks::Element *);
    void static hash(Goldilocks::Element (&state)[CAPACITY], const Goldilocks::Element (&input)[SPONGE_WIDTH]);
    void static linear_hash(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t size);
    void static merkletree_avx(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, int nThreads = 0, uint64_t dim = 1);
    void static merkletree_batch_avx(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, uint64_t batch_size, int nThreads = 0, uint64_t dim = 1);

#ifdef __AVX512__
    // Vectorized AVX512:
    void static hash_full_result_avx512(Goldilocks::Element *, const Goldilocks::Element *);
    void static hash_avx512(Goldilocks::Element (&state)[2 * CAPACITY], const Goldilocks::Element (&input)[2 * SPONGE_WIDTH]);
    void static linear_hash_avx512(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t size);
    void static merkletree_avx512(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, int nThreads = 0, uint64_t dim = 1);
    void static merkletree_batch_avx512(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, uint64_t batch_size, int nThreads = 0, uint64_t dim = 1);
#endif
};

// WRAPPERS

inline void PoseidonGoldilocks::merkletree(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, int nThreads, uint64_t dim)
{
#ifdef __AVX512__
    merkletree_avx512(tree, input, num_cols, num_rows, nThreads, dim);
#else
    merkletree_avx(tree, input, num_cols, num_rows, nThreads, dim);
#endif
}
inline void PoseidonGoldilocks::merkletree_batch(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, uint64_t batch_size, int nThreads, uint64_t dim)
{
#ifdef __AVX512__
    merkletree_batch_avx512(tree, input, num_cols, num_rows, batch_size, nThreads, dim);
#else
    merkletree_batch_avx(tree, input, num_cols, num_rows, batch_size, nThreads, dim);
#endif
}

inline void PoseidonGoldilocks::pow7(Goldilocks::Element &x)
{
    Goldilocks::Element x2 = x * x;
    Goldilocks::Element x3 = x * x2;
    Goldilocks::Element x4 = x2 * x2;
    x = x3 * x4;
};
inline void PoseidonGoldilocks::pow7_(Goldilocks::Element *x)
{
    Goldilocks::Element x2[SPONGE_WIDTH], x3[SPONGE_WIDTH], x4[SPONGE_WIDTH];
    for (int i = 0; i < SPONGE_WIDTH; ++i)
    {
        x2[i] = x[i] * x[i];
        x3[i] = x[i] * x2[i];
        x4[i] = x2[i] * x2[i];
        x[i] = x3[i] * x4[i];
    }
};

inline void PoseidonGoldilocks::add_(Goldilocks::Element *x, const Goldilocks::Element C[SPONGE_WIDTH])
{
    for (int i = 0; i < SPONGE_WIDTH; ++i)
    {
        x[i] = x[i] + C[i];
    }
}
inline void PoseidonGoldilocks::prod_(Goldilocks::Element *x, const Goldilocks::Element alpha, const Goldilocks::Element C[SPONGE_WIDTH])
{
    for (int i = 0; i < SPONGE_WIDTH; ++i)
    {
        x[i] = alpha * C[i];
    }
}

inline void PoseidonGoldilocks::pow7add_(Goldilocks::Element *x, const Goldilocks::Element C[SPONGE_WIDTH])
{
    Goldilocks::Element x2[SPONGE_WIDTH], x3[SPONGE_WIDTH], x4[SPONGE_WIDTH];
    for (int i = 0; i < SPONGE_WIDTH; ++i)
    {
        x2[i] = x[i] * x[i];
        x3[i] = x[i] * x2[i];
        x4[i] = x2[i] * x2[i];
        x[i] = x3[i] * x4[i];
        x[i] = x[i] + C[i];
    }
};
inline Goldilocks::Element PoseidonGoldilocks::dot_(Goldilocks::Element *x, const Goldilocks::Element C[SPONGE_WIDTH])
{
    Goldilocks::Element s0 = x[0] * C[0];
    for (int i = 1; i < SPONGE_WIDTH; i++)
    {
        s0 = s0 + x[i] * C[i];
    }
    return s0;
};

// rick: check transpose access to matrix
inline void PoseidonGoldilocks::mvp_(Goldilocks::Element *state, const Goldilocks::Element mat[SPONGE_WIDTH][SPONGE_WIDTH])
{
    Goldilocks::Element old_state[SPONGE_WIDTH];
    std::memcpy(old_state, state, sizeof(Goldilocks::Element) * SPONGE_WIDTH);

    for (int i = 0; i < SPONGE_WIDTH; i++)
    {

        state[i] = mat[0][i] * old_state[0];
        for (int j = 1; j < SPONGE_WIDTH; j++)
        {
            state[i] = state[i] + (mat[j][i] * old_state[j]);
        }
    }
};

inline void PoseidonGoldilocks::hash_seq(Goldilocks::Element (&state)[CAPACITY], Goldilocks::Element const (&input)[SPONGE_WIDTH])
{
    Goldilocks::Element aux[SPONGE_WIDTH];
    hash_full_result_seq(aux, input);
    std::memcpy(state, aux, CAPACITY * sizeof(Goldilocks::Element));
}

#include "poseidon_goldilocks_avx.hpp"

#ifdef __AVX512__
#include "poseidon_goldilocks_avx512.hpp"
#endif
#endif