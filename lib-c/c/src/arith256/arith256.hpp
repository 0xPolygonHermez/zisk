#ifndef ARITH256_HPP
#define ARITH256_HPP

#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

// Computes d = (a * b) + c
int Arith256 (
    const uint64_t * a,  // 4 x 64 bits
    const uint64_t * b,  // 4 x 64 bits
    const uint64_t * c,  // 4 x 64 bits
    uint64_t * dl, // 4 x 64 bits
    uint64_t * dh // 4 x 64 bits
);

// Computes d = ((a * b) + c) % module
int Arith256Mod (
    const uint64_t * a,  // 4 x 64 bits
    const uint64_t * b,  // 4 x 64 bits
    const uint64_t * c,  // 4 x 64 bits
    const uint64_t * module,  // 4 x 64 bits
    uint64_t * d // 4 x 64 bits
);

int FastArith256(
    const uint64_t * _a,  // 4 x 64 bits (input: a)
    const uint64_t * _b,  // 4 x 64 bits (input: b)
    const uint64_t * _c,  // 4 x 64 bits (input: c)
          uint64_t * _dl, // 4 x 64 bits (output: less significant 64-bit result)
          uint64_t * _dh  // 4 x 64 bits (output: most significant 64-bit result)
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
