#ifndef POSEIDON2_GOLDILOCKS_HPP
#define POSEIDON2_GOLDILOCKS_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void poseidon2_hash(uint64_t *state);

#ifdef __cplusplus
} // extern "C"
#endif

#endif