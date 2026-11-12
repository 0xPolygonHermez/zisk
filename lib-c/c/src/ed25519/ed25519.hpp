#ifndef ED25519_HPP
#define ED25519_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/************************************/
/* Ed25519 twisted-Edwards add      */
/************************************/

// Complete point addition on edwards25519 (-x^2 + y^2 = 1 + d*x^2*y^2, d = -121665/121666)
// over Fp, p = 2^255 - 19. Points are affine [x, y], each coordinate 4 x 64-bit LE limbs.
// Input coordinates are taken mod p; the result is canonical. The same formula doubles
// (p1 == p2).
//
//   tau = x1*x2*y1*y2
//   x3  = (x1*y2 + y1*x2) / (1 + d*tau)
//   y3  = (y1*y2 + x1*x2) / (1 - d*tau)
//
// Returns 0 on success, -1 if a denominator is zero (cannot happen for valid
// curve points since d and -d are non-residues mod p).

int Ed25519AddP (
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * p3  // 8 x 64 bits
);

// Free-input hint for decompression: square root of u/v mod p (fcall
// FCALL_ED25519_FP_SQRT_RATIO_ID). r = [is_square, root[4]], root canonical and even:
//   is_square = 1: v*root^2 = u   (root = 0 when u = 0)
//   is_square = 0: v*root^2 = 2u  (2 is a non-residue mod p, so u/v is not a square)
// v = 0 with u != 0 has no valid answer: r = [0, 0] (the guest check then fails).
int Ed25519FpSqrtRatio (
    const uint64_t * u, // 4 x 64 bits
    const uint64_t * v, // 4 x 64 bits
    uint64_t * r  // 5 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
