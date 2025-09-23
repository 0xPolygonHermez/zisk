#ifndef ARITH384_HPP
#define ARITH384_HPP

#ifdef __cplusplus
extern "C" {
#endif

// Computes d = (a * b) + c
int Arith384 (
    const unsigned long * a,  // 6 x 64 bits
    const unsigned long * b,  // 6 x 64 bits
    const unsigned long * c,  // 6 x 64 bits
    unsigned long * dl, // 6 x 64 bits
    unsigned long * dh // 6 x 64 bits
);

// Computes d = ((a * b) + c) % module
int Arith384Mod (
    const unsigned long * a,  // 6 x 64 bits
    const unsigned long * b,  // 6 x 64 bits
    const unsigned long * c,  // 6 x 64 bits
    const unsigned long * module,  // 6 x 64 bits
    unsigned long * d // 6 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
