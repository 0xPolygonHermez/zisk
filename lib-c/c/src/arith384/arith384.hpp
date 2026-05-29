#ifndef ARITH384_HPP
#define ARITH384_HPP

#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

// Computes d = (a * b) + c
int Arith384 (
    const uint64_t * a,  // 6 x 64 bits
    const uint64_t * b,  // 6 x 64 bits
    const uint64_t * c,  // 6 x 64 bits
    uint64_t * dl, // 6 x 64 bits
    uint64_t * dh // 6 x 64 bits
);

// Computes d = ((a * b) + c) % module
int Arith384Mod (
    const uint64_t * a,  // 6 x 64 bits
    const uint64_t * b,  // 6 x 64 bits
    const uint64_t * c,  // 6 x 64 bits
    const uint64_t * module,  // 6 x 64 bits
    uint64_t * d // 6 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
