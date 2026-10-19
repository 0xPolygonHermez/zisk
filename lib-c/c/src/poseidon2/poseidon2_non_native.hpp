#ifndef POSEIDON2_NON_NATIVE_HPP
#define POSEIDON2_NON_NATIVE_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Poseidon2 over 31-bit prime fields, on a state packed two lanes per little-endian uint64_t, each
// lane in the instance's memory form (Montgomery for KoalaBear). Each returns 0, or -1 (state
// untouched) if a lane is not canonical.

// KoalaBear, width 16 (precompiles/helpers/src/poseidon2/koalabear.rs): 8 words
int poseidon2_koalabear(uint64_t *state);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
