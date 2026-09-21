#ifndef BABYJUBJUB_HPP
#define BABYJUBJUB_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/************************************/
/* BabyJubJub twisted-Edwards add   */
/************************************/

// Complete point addition on BabyJubJub (a*x^2 + y^2 = 1 + d*x^2*y^2, a = 168700,
// d = 168696) over the BN254 scalar field Fr. Points are affine [x, y], each
// coordinate 4 x 64-bit LE limbs. The same formula doubles (p1 == p2).
//
//   tau = x1*x2*y1*y2
//   x3  = (x1*y2 + y1*x2) / (1 + d*tau)
//   y3  = (y1*y2 - a*x1*x2) / (1 - d*tau)
//
// Returns 0 on success, -1 if a denominator is zero (cannot happen for valid
// curve points since d and a*d are non-residues in Fr).

int BabyJubJubAdd (
    const uint64_t * x1, // 4 x 64 bits
    const uint64_t * y1, // 4 x 64 bits
    const uint64_t * x2, // 4 x 64 bits
    const uint64_t * y2, // 4 x 64 bits
    uint64_t * x3, // 4 x 64 bits
    uint64_t * y3  // 4 x 64 bits
);

int BabyJubJubAddP (
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * p3  // 8 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
