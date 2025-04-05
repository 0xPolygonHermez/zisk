#ifndef ARITH256_HPP
#define ARITH256_HPP

#ifdef __cplusplus
extern "C" {
#endif

// Computes d = (a * b) + c
int Arith256 (
    const unsigned long * a,  // 4 x 64 bits
    const unsigned long * b,  // 4 x 64 bits
    const unsigned long * c,  // 4 x 64 bits
    unsigned long * dl, // 4 x 64 bits
    unsigned long * dh // 4 x 64 bits
);

// Computes d = ((a * b) + c) % module
int Arith256Mod (
    const unsigned long * a,  // 4 x 64 bits
    const unsigned long * b,  // 4 x 64 bits
    const unsigned long * c,  // 4 x 64 bits
    const unsigned long * module,  // 4 x 64 bits
    unsigned long * d // 4 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
