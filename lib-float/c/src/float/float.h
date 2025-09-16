#ifndef ZISK_FLOAT_HPP
#define ZISK_FLOAT_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define uint64_t unsigned long long

// System address where the floating-point registers are mapped
const uint64_t SYS_ADDR = 0xa0000000;
const uint64_t REG_FIRST = SYS_ADDR;
const uint64_t FREG_OFFSET = 40;
const uint64_t FREG_FIRST = SYS_ADDR + FREG_OFFSET * 8;
const uint64_t FREG_F0 = FREG_FIRST;
const uint64_t FREG_INST = FREG_FIRST + 33 * 8; // Floating-point instruction register (finst)
const uint64_t FREG_X0 = FREG_FIRST + 35 * 8; // Integer register backup for floating-point instructions (fX0)

const uint64_t CSR_ADDR = SYS_ADDR + 0x8000;
const uint64_t FREG_CSR = CSR_ADDR + 3 * 8;


#define fregs ((volatile uint64_t *)FREG_F0)
#define fregs_x ((volatile uint64_t *)FREG_X0)
#define fcsr (*(volatile uint32_t *)FREG_CSR)

static uint64_t myvalue = 0x3ff3333333333333; // 1.7

// Negate a float by flipping its sign bit(s)
const uint64_t F64_SIGN_BIT_MASK = 0x8000000000000000;
const uint64_t F32_SIGN_BIT_MASK = 0x80000000;
#define NEG64(x) ((x) ^ F64_SIGN_BIT_MASK)
#define NEG32(x) ((x) ^ F32_SIGN_BIT_MASK)
const uint64_t F64_EXPONENT_MASK = 0x7FF0000000000000;
const uint64_t F32_EXPONENT_MASK = 0x7F800000;
const uint64_t F64_MANTISSA_MASK = 0x000FFFFFFFFFFFFF;
const uint64_t F32_MANTISSA_MASK = 0x007FFFFF;
const uint64_t F64_QUIET_NAN_MASK = 0x0008000000000000;
const uint64_t F32_QUIET_NAN_MASK = 0x00400000;

// Plus and minus infinity in IEEE 754 format
const uint64_t F64_MINUS_INFINITE = 0xFFF0000000000000;
const uint64_t F64_PLUS_INFINITE = 0x7FF0000000000000;
const uint32_t F32_MINUS_INFINITE = 0xFF800000;
const uint32_t F32_PLUS_INFINITE = 0x7F800000;

// Plus and minus zero in IEEE 754 format
const uint64_t F64_MINUS_ZERO = 0x8000000000000000;
const uint64_t F64_PLUS_ZERO = 0x0000000000000000;
const uint32_t F32_MINUS_ZERO = 0x80000000;
const uint32_t F32_PLUS_ZERO = 0x00000000;

// 1.0 and 0.0 in IEEE 754 format
const uint64_t F64_ONE = 0x3FF0000000000000;
const uint64_t F32_ONE = 0x3F800000;
const uint64_t F64_ZERO = 0x0000000000000000;
const uint32_t F32_ZERO = 0x00000000;

void _zisk_float (void);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
