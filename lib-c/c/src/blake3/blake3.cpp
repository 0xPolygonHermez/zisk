#include "blake3.hpp"
#include <cstddef>

/// Message word permutation schedule (cumulative BLAKE3 permutations, one row per round)
const size_t SIGMA[7][16] = {
    {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15},
    {2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8},
    {3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1},
    {10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6},
    {12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4},
    {9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7},
    {11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13},
};

/// Rotation constants for G function
const uint32_t R1 = 16;
const uint32_t R2 = 12;
const uint32_t R3 = 8;
const uint32_t R4 = 7;

// U32 rotate right function
static inline uint32_t rotate_right_32(uint32_t x, unsigned int n) {
    n &= 31;
    return (x >> n) | (x << (32 - n));
}

/// G mixing function
///
/// The G function mixes two input words `x` and `y` from the message block into the state.
/// It operates on 4 state words: v[a], v[b], v[c], v[d]
static inline void g(uint32_t v[16], size_t a, size_t b, size_t c, size_t d, uint32_t x, uint32_t y) {
    uint32_t va = v[a];
    uint32_t vb = v[b];
    uint32_t vc = v[c];
    uint32_t vd = v[d];

    va = va + vb + x;
    vd = rotate_right_32(vd ^ va, R1);
    vc = vc + vd;
    vb = rotate_right_32(vb ^ vc, R2);

    va = va + vb + y;
    vd = rotate_right_32(vd ^ va, R3);
    vc = vc + vd;
    vb = rotate_right_32(vb ^ vc, R4);

    v[a] = va;
    v[b] = vb;
    v[c] = vc;
    v[d] = vd;
}

/// BLAKE3 round function
static void blake3_round(uint32_t v[16], const uint32_t m[16], size_t round) {
    // Message word selection permutation for this round
    const size_t* s = SIGMA[round];

    // Column step
    g(v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
    g(v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
    g(v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
    g(v, 3, 7, 11, 15, m[s[6]], m[s[7]]);

    // Diagonal step
    g(v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
    g(v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
    g(v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
    g(v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
}

/// BLAKE3 simplified compression function: the 7-round permutation, no feed-forward
void blake3_f(uint32_t v[16], const uint32_t m[16]) {
    for (size_t round = 0; round < 7; round++) {
        blake3_round(v, m, round);
    }
}
