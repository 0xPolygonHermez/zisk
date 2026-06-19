#ifndef SECP256R1_HPP
#define SECP256R1_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int secp256r1_add_point_ec (
    uint64_t dbl,
    const uint64_t * x1, // 4 x 64 bits
    const uint64_t * y1, // 4 x 64 bits
    const uint64_t * x2, // 4 x 64 bits
    const uint64_t * y2, // 4 x 64 bits
    uint64_t * x3, // 4 x 64 bits
    uint64_t * y3  // 4 x 64 bits
);

int secp256r1_add_point_ecp (
    const uint64_t dbl,
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * p3  // 8 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
