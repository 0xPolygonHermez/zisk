#ifndef POSEIDON2_GOLDILOCKS
#define POSEIDON2_GOLDILOCKS

#include <vector>
#include "poseidon2_goldilocks_constants.hpp"
#include "goldilocks_base_field.hpp"

#define WIDTH 16

inline void pow7(Goldilocks::Element &x)
{
    Goldilocks::Element x2 = x * x;
    Goldilocks::Element x3 = x * x2;
    Goldilocks::Element x4 = x2 * x2;
    x = x3 * x4;
};

inline void add_(Goldilocks::Element &x, const Goldilocks::Element *st)
{
    for (int i = 0; i < WIDTH; ++i)
    {
        x = x + st[i];
    }
}
inline void prodadd_(Goldilocks::Element *x, const Goldilocks::Element *D, const Goldilocks::Element &sum)
{
    for (int i = 0; i < WIDTH; ++i)
    {
        x[i] = x[i]*D[i] + sum;
    }
}

inline void pow7add_(Goldilocks::Element *x, const Goldilocks::Element *C)
{
    Goldilocks::Element x2[WIDTH], x3[WIDTH], x4[WIDTH];
    
    for (int i = 0; i < WIDTH; ++i)
    {
        Goldilocks::Element xi = x[i] + C[i];
        x2[i] = xi * xi;
        x3[i] = xi * x2[i];
        x4[i] = x2[i] * x2[i];
        x[i] = x3[i] * x4[i];
    }
};

inline void matmul_m4_(Goldilocks::Element *x) {
    Goldilocks::Element t0 = x[0] + x[1];
    Goldilocks::Element t1 = x[2] + x[3];
    Goldilocks::Element t2 = x[1] + x[1] + t1;
    Goldilocks::Element t3 = x[3] + x[3] + t0;
    Goldilocks::Element t1_2 = t1 + t1;
    Goldilocks::Element t0_2 = t0 + t0;
    Goldilocks::Element t4 = t1_2 + t1_2 + t3;
    Goldilocks::Element t5 = t0_2 + t0_2 + t2;
    Goldilocks::Element t6 = t3 + t5;
    Goldilocks::Element t7 = t2 + t4;
    
    x[0] = t6;
    x[1] = t5;
    x[2] = t7;
    x[3] = t4;
}

inline void matmul_external_(Goldilocks::Element *x) {
    for (int i = 0; i < WIDTH/4; ++i) {
        matmul_m4_(&x[i*4]);
    }
    
    Goldilocks::Element stored[4] = {Goldilocks::zero(), Goldilocks::zero(), Goldilocks::zero(), Goldilocks::zero()};

    for(int i = 0; i < 4; ++i) {
        for (int j = 0; j < WIDTH/4; ++j) {
            stored[i] = stored[i] + x[j*4 + i];
        }
    }
    
    for (int i = 0; i < WIDTH; ++i)
    {
        x[i] = x[i] + stored[i % 4];
    }
}

void Poseidon2(Goldilocks::Element *state)
{   
    const Goldilocks::Element *RC = Poseidon2GoldilocksConstants::RC;
    const Goldilocks::Element *D = Poseidon2GoldilocksConstants::DIAG;

    matmul_external_(state);
    
    for (int r = 0; r < 4; r++)
    {
        pow7add_(state, &(RC[WIDTH * r]));
        matmul_external_(state);
    }

    for (int r = 0; r < 22; r++)
    {
        state[0] = state[0] + RC[4 * WIDTH + r];
        pow7(state[0]);
        Goldilocks::Element sum_ = Goldilocks::zero();
        add_(sum_, state);
        prodadd_(state, D, sum_);
    }

    for (int r = 0; r < 4; r++)
    {
        pow7add_(state, &(RC[4 * WIDTH + 22 + r * WIDTH]));
        matmul_external_(state);
    }
}

#ifdef __cplusplus
extern "C" {
#endif

void poseidon2_hash(uint64_t *state)
{
    Goldilocks::Element stateGL[16];
    for(uint64_t i = 0; i < 16; ++i) {
        stateGL[i] = Goldilocks::fromU64(state[i]);
    }
    Poseidon2(stateGL);

    for(uint64_t i = 0; i < WIDTH; ++i) {
        state[i] = Goldilocks::toU64(stateGL[i]);
    }
}

#ifdef __cplusplus
} // extern "C"
#endif

#endif