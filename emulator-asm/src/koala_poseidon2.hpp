// KoalaBear Poseidon2 for the ASM emulator. Same packed canonical ABI as
// definitions/src/koala_poseidon2.rs; the two are compared by definitions/runtime-tests.
#pragma once
#include <stdbool.h>
#include "koala_poseidon2_parameters.hpp"

static inline uint32_t koala_add(uint32_t a, uint32_t b) {
    return ((uint64_t)a + b) % 2130706433;
}

static inline uint32_t koala_mul(uint32_t a, uint32_t b) {
    return ((uint64_t)a * b) % 2130706433;
}

static inline uint32_t koala_cube(uint32_t a) { return koala_mul(koala_mul(a, a), a); }

static inline void koala_matrix(uint32_t state[16], const uint32_t matrix[16][16]) {
    uint32_t input[16];
    for (unsigned i = 0; i < 16; i++) input[i] = state[i];
    for (unsigned i = 0; i < 16; i++) {
        state[i] = 0;
        for (unsigned j = 0; j < 16; j++) state[i] = koala_add(state[i], koala_mul(input[j], matrix[i][j]));
    }
}

static inline void koala_full_round(uint32_t state[16], const uint32_t constants[16]) {
    for (unsigned i = 0; i < 16; i++) state[i] = koala_cube(koala_add(state[i], constants[i]));
    koala_matrix(state, KOALA_EXTERNAL);
}

// Permutes in place. Returns false, leaving the words untouched, on a noncanonical lane.
static inline bool koala_poseidon2_packed(uint64_t words[8]) {
    uint32_t state[16];
    for (unsigned i = 0; i < 16; i++) {
        state[i] = words[i / 2] >> (32 * (i % 2));
        if (state[i] >= 2130706433) return false;
    }
    koala_matrix(state, KOALA_EXTERNAL);
    for (unsigned i = 0; i < 4; i++) koala_full_round(state, KOALA_BEGIN[i]);
    for (unsigned i = 0; i < 20; i++) {
        state[0] = koala_cube(koala_add(state[0], KOALA_PARTIAL[i]));
        koala_matrix(state, KOALA_INTERNAL);
    }
    for (unsigned i = 0; i < 4; i++) koala_full_round(state, KOALA_END[i]);
    for (unsigned i = 0; i < 8; i++) words[i] = (uint64_t)state[2 * i] | ((uint64_t)state[2 * i + 1] << 32);
    return true;
}
