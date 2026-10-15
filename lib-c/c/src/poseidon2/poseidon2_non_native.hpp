#ifndef POSEIDON2_NON_NATIVE_HPP
#define POSEIDON2_NON_NATIVE_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Poseidon2 over 31-bit prime fields, on a state of canonical elements packed two per
// little-endian uint64_t. Each returns 0, or -1 (state untouched) if an element is not canonical.

// KoalaBear, width 16, SP1's parameters (`my_kb_16_perm`): 8 words
int poseidon2_koalabear(uint64_t *state);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
