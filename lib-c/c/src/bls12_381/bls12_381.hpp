#ifndef BLS12_381_HPP
#define BLS12_381_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*******************/
/* BN254 curve add */
/*******************/

int BLS12_381CurveAdd (
    const uint64_t * x1, // 6 x 64 bits
    const uint64_t * y1, // 6 x 64 bits
    const uint64_t * x2, // 6 x 64 bits
    const uint64_t * y2, // 6 x 64 bits
    uint64_t * x3, // 6 x 64 bits
    uint64_t * y3  // 6 x 64 bits
);

int BLS12_381CurveAddP (
    const uint64_t * p1, // 12 x 64 bits
    const uint64_t * p2, // 12 x 64 bits
    uint64_t * p3  // 12 x 64 bits
);

/**************************/
/* BLS12_381 curve double */
/**************************/

int BLS12_381CurveDbl (
    const uint64_t * x1, // 6 x 64 bits
    const uint64_t * y1, // 6 x 64 bits
    uint64_t * x2, // 6 x 64 bits
    uint64_t * y3  // 6 x 64 bits
);

int BLS12_381CurveDblP (
    const uint64_t * p1, // 12 x 64 bits
    uint64_t * p2  // 12 x 64 bits
);

/*************************/
/* BLS12_381 complex add */
/*************************/

int BLS12_381ComplexAdd (
    const uint64_t * x1, // 6 x 64 bits
    const uint64_t * y1, // 6 x 64 bits
    const uint64_t * x2, // 6 x 64 bits
    const uint64_t * y2, // 6 x 64 bits
    uint64_t * x3, // 6 x 64 bits
    uint64_t * y3  // 6 x 64 bits
);

int BLS12_381ComplexAddP (
    const uint64_t * p1, // 12 x 64 bits
    const uint64_t * p2, // 12 x 64 bits
    uint64_t * p3  // 12 x 64 bits
);

/*************************/
/* BLS12_381 complex sub */
/*************************/

int BLS12_381ComplexSub (
    const uint64_t * x1, // 6 x 64 bits
    const uint64_t * y1, // 6 x 64 bits
    const uint64_t * x2, // 6 x 64 bits
    const uint64_t * y2, // 6 x 64 bits
    uint64_t * x3, // 6 x 64 bits
    uint64_t * y3  // 6 x 64 bits
);

int BLS12_381ComplexSubP (
    const uint64_t * p1, // 12 x 64 bits
    const uint64_t * p2, // 12 x 64 bits
    uint64_t * p3  // 12 x 64 bits
);

/*************************/
/* BLS12_381 complex mul */
/*************************/

int BLS12_381ComplexMul (
    const uint64_t * x1, // 6 x 64 bits
    const uint64_t * y1, // 6 x 64 bits
    const uint64_t * x2, // 6 x 64 bits
    const uint64_t * y2, // 6 x 64 bits
    uint64_t * x3, // 6 x 64 bits
    uint64_t * y3  // 6 x 64 bits
);

int BLS12_381ComplexMulP (
    const uint64_t * p1, // 12 x 64 bits
    const uint64_t * p2, // 12 x 64 bits
    uint64_t * p3  // 12 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
