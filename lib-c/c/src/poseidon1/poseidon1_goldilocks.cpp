#ifndef POSEIDON1_GOLDILOCKS
#define POSEIDON1_GOLDILOCKS

#include "poseidon1_goldilocks_constants.hpp"
#include "goldilocks_base_field.hpp"

#define WIDTH 16
#define HALF_FULL_ROUNDS 4
#define N_PARTIAL_ROUNDS 22

inline void pow7_(Goldilocks::Element &x)
{
    Goldilocks::Element x2 = x * x;
    Goldilocks::Element x3 = x * x2;
    Goldilocks::Element x4 = x2 * x2;
    x = x3 * x4;
}

// state[i] = sum_j old[j] * mat[j*W + i]
inline void matmul_(const Goldilocks::Element *mat, Goldilocks::Element *state)
{
    Goldilocks::Element old[WIDTH];
    for (int i = 0; i < WIDTH; ++i)
    {
        old[i] = state[i];
    }
    for (int i = 0; i < WIDTH; ++i)
    {
        Goldilocks::Element sum = old[0] * mat[i];
        for (int j = 1; j < WIDTH; ++j)
        {
            sum = sum + old[j] * mat[j * WIDTH + i];
        }
        state[i] = sum;
    }
}

void Poseidon1(Goldilocks::Element *state)
{
    const Goldilocks::Element *C = Poseidon1GoldilocksConstants::C;
    const Goldilocks::Element *M = Poseidon1GoldilocksConstants::M;
    const Goldilocks::Element *P = Poseidon1GoldilocksConstants::P;
    const Goldilocks::Element *S = Poseidon1GoldilocksConstants::S;

    // Initial ARC: state += C[0..W]
    for (int i = 0; i < WIDTH; ++i)
    {
        state[i] = state[i] + C[i];
    }

    // First HALF_FULL_ROUNDS-1 full rounds with M matrix.
    for (int r = 0; r < HALF_FULL_ROUNDS - 1; ++r)
    {
        for (int i = 0; i < WIDTH; ++i)
        {
            pow7_(state[i]);
            state[i] = state[i] + C[(r + 1) * WIDTH + i];
        }
        matmul_(M, state);
    }

    // Transition full round with P matrix.
    for (int i = 0; i < WIDTH; ++i)
    {
        pow7_(state[i]);
        state[i] = state[i] + C[HALF_FULL_ROUNDS * WIDTH + i];
    }
    matmul_(P, state);

    // 22 partial rounds with sparse S matrices.
    const int partial_c_base = (HALF_FULL_ROUNDS + 1) * WIDTH;
    const int stride = 2 * WIDTH - 1;
    for (int r = 0; r < N_PARTIAL_ROUNDS; ++r)
    {
        pow7_(state[0]);
        state[0] = state[0] + C[partial_c_base + r];

        const int s_base = stride * r;

        // s0 = sum_j state[j] * S[s_base + j]
        Goldilocks::Element s0 = state[0] * S[s_base];
        for (int j = 1; j < WIDTH; ++j)
        {
            s0 = s0 + state[j] * S[s_base + j];
        }

        // state[t] += state[0] * S[s_base + (W-1) + t] for t in 1..W
        Goldilocks::Element s0_active = state[0];
        for (int t = 1; t < WIDTH; ++t)
        {
            state[t] = state[t] + s0_active * S[s_base + (WIDTH - 1) + t];
        }

        state[0] = s0;
    }

    // Last HALF_FULL_ROUNDS-1 full rounds with M matrix.
    const int post_partial_base = (HALF_FULL_ROUNDS + 1) * WIDTH + N_PARTIAL_ROUNDS;
    for (int r = 0; r < HALF_FULL_ROUNDS - 1; ++r)
    {
        for (int i = 0; i < WIDTH; ++i)
        {
            pow7_(state[i]);
            state[i] = state[i] + C[post_partial_base + r * WIDTH + i];
        }
        matmul_(M, state);
    }

    // Final round: pow7 + M (no ARC).
    for (int i = 0; i < WIDTH; ++i)
    {
        pow7_(state[i]);
    }
    matmul_(M, state);
}

#ifdef __cplusplus
extern "C" {
#endif

void poseidon1_hash(uint64_t *state)
{
    Goldilocks::Element stateGL[WIDTH];
    for (uint64_t i = 0; i < WIDTH; ++i)
    {
        stateGL[i] = Goldilocks::fromU64(state[i]);
    }
    Poseidon1(stateGL);

    for (uint64_t i = 0; i < WIDTH; ++i)
    {
        state[i] = Goldilocks::toU64(stateGL[i]);
    }
}

#ifdef __cplusplus
} // extern "C"
#endif

#endif
