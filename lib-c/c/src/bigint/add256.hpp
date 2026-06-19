#ifndef ADD256_HPP
#define ADD256_HPP

// This header is part of libziskc's C ABI and is included by C translation
// units too (e.g. the asm emulator's emu.c, built with gcc as C), so it must
// not pull in the C++-only <cstdint>.
#ifdef __cplusplus
#include <cstdint>
#else
#include <stdint.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

// Computes d = (a * b) + c
int Add256 (
    const uint64_t * _a,  // 4 x 64 bits (input: &uint64_t a[4])
    const uint64_t * _b,  // 4 x 64 bits (input: &uint64_t b[4])
    const uint64_t cin,   // 64 bits (input: uint64_t carry in)
          uint64_t * _c   // 4 x 64 bits (output: &uint64_t c[4])
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
