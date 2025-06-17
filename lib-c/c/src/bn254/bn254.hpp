#ifndef BN254_HPP
#define BN254_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*******************/
/* BN254 curve add */
/*******************/

int BN254CurveAdd (
    const uint64_t * x1, // 4 x 64 bits
    const uint64_t * y1, // 4 x 64 bits
    const uint64_t * x2, // 4 x 64 bits
    const uint64_t * y2, // 4 x 64 bits
    uint64_t * x3, // 4 x 64 bits
    uint64_t * y3  // 4 x 64 bits
);

int BN254CurveAddP (
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * p3  // 8 x 64 bits
);

/**********************/
/* BN254 curve double */
/**********************/

int BN254CurveDbl (
    const uint64_t * x1, // 4 x 64 bits
    const uint64_t * y1, // 4 x 64 bits
    uint64_t * x2, // 4 x 64 bits
    uint64_t * y3  // 4 x 64 bits
);

int BN254CurveDblP (
    const uint64_t * p1, // 8 x 64 bits
    uint64_t * p2  // 8 x 64 bits
);

/*********************/
/* BN254 complex add */
/*********************/

int BN254ComplexAdd (
    const uint64_t * x1, // 4 x 64 bits
    const uint64_t * y1, // 4 x 64 bits
    const uint64_t * x2, // 4 x 64 bits
    const uint64_t * y2, // 4 x 64 bits
    uint64_t * x3, // 4 x 64 bits
    uint64_t * y3  // 4 x 64 bits
);

int BN254ComplexAddP (
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * p3  // 8 x 64 bits
);

/*********************/
/* BN254 complex sub */
/*********************/

int BN254ComplexSub (
    const uint64_t * x1, // 4 x 64 bits
    const uint64_t * y1, // 4 x 64 bits
    const uint64_t * x2, // 4 x 64 bits
    const uint64_t * y2, // 4 x 64 bits
    uint64_t * x3, // 4 x 64 bits
    uint64_t * y3  // 4 x 64 bits
);

int BN254ComplexSubP (
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * p3  // 8 x 64 bits
);

/*********************/
/* BN254 complex mul */
/*********************/

int BN254ComplexMul (
    const uint64_t * x1, // 4 x 64 bits
    const uint64_t * y1, // 4 x 64 bits
    const uint64_t * x2, // 4 x 64 bits
    const uint64_t * y2, // 4 x 64 bits
    uint64_t * x3, // 4 x 64 bits
    uint64_t * y3  // 4 x 64 bits
);

int BN254ComplexMulP (
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * p3  // 8 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
