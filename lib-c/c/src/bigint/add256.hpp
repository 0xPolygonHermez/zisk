#ifndef ADD256_HPP
#define ADD256_HPP

#ifdef __cplusplus
extern "C" {
#endif

// Computes d = (a * b) + c
int Add256 (
    const unsigned long * _a,  // 4 x 64 bits (input: &uint64_t a[4])
    const unsigned long * _b,  // 4 x 64 bits (input: &uint64_t b[4])
    const unsigned long cin,   // 64 bits (input: uint64_t carry in)
          unsigned long * _c   // 4 x 64 bits (output: &uint64_t c[4])
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
