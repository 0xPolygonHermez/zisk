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

int FastArith256(
    const unsigned long * _a,  // 4 x 64 bits (input: a)
    const unsigned long * _b,  // 4 x 64 bits (input: b)  
    const unsigned long * _c,  // 4 x 64 bits (input: c)
          unsigned long * _dl, // 4 x 64 bits (output: less significant 64-bit result)
          unsigned long * _dh  // 4 x 64 bits (output: most significant 64-bit result)
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
