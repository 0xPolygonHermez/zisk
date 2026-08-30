#ifndef LIB_C_BLAKE2_HPP
#define LIB_C_BLAKE2_HPP

#include <stdint.h> // uint64_t

#ifdef __cplusplus
extern "C" {
#endif

void blake2b_round(uint64_t v[16], const uint64_t m[16], uint64_t round);

// BLAKE2s round over 32-bit words. Each word occupies its own u64 slot with a
// zero high half, matching the layout the Blake2sr AIR enforces.
void blake2s_round(uint64_t v[16], const uint64_t m[16], uint64_t round);

#ifdef __cplusplus
}
#endif

#endif // LIB_C_BLAKE2_HPP