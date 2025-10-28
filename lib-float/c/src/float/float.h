#ifndef ZISK_FLOAT_HPP
#define ZISK_FLOAT_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define uint64_t unsigned long long

// System address where the floating-point registers are mapped
#define SYS_ADDR 0xa0000000
#define REG_FIRST SYS_ADDR
#define FREG_FIRST (SYS_ADDR + 0x1000)
#define FREG_F0 FREG_FIRST
#define FREG_INST (FREG_FIRST + 33 * 8) // Floating-point instruction register (finst)
#define FREG_X0 (FREG_FIRST + 35 * 8) // Integer register backup for floating-point instructions (fX0)
#define CSR_ADDR (SYS_ADDR + 0x8000)
#define FREG_CSR (CSR_ADDR + 3 * 8)

// Array-like access to floating-point registers and control/status register: fregs[3], etc
#define fregs ((volatile uint64_t *)FREG_F0)
#define fregs_x ((volatile uint64_t *)FREG_X0)
#define fcsr (*(volatile uint32_t *)FREG_CSR)

// Sign, exponent and mantissa masks for single and double precision floats

/* IEEE 754 DoubleFloating Point Representation:

MSB                                                                         LSB
+-----+------------------------+---------------------------------------------------+
|  1  |           11           |                      52                           |
+-----+------------------------+---------------------------------------------------+
|Sign |        Exponent        |                   Fraction                        |
|     |        (E - 1023)      |               (Significand - 1)                   |
+-----+------------------------+---------------------------------------------------+
  63      62                52   51                                               0 
  
  Value = (-1)^sign × 1.fraction × 2^(exponent - 1023)    ~15-16 decimal digits
*/

#define F64_SIGN_BIT_MASK  0x8000000000000000
#define F64_EXPONENT_MASK  0x7FF0000000000000
#define F64_MANTISSA_MASK  0x000FFFFFFFFFFFFF
#define F64_QUIET_NAN_MASK 0x0008000000000000

/* IEEE 754 Floating Point Representation:

MSB                                                         LSB
+-----+------------------------+-------------------------------+
|  1  |           8            |              23               |
+-----+------------------------+-------------------------------+
|Sign |        Exponent        |           Fraction            |
|     |        (E - 127)       |       (Significand - 1)       |
+-----+------------------------+-------------------------------+
  31      30                23   22                           0
  
  Value = (-1)^sign × 1.fraction × 2^(exponent - 127)     ~7 decimal digits
*/
  
#define F32_SIGN_BIT_MASK  0xFFFFFFFF80000000
#define F32_EXPONENT_MASK  0x7F800000
#define F32_MANTISSA_MASK  0x007FFFFF
#define F32_QUIET_NAN_MASK 0x00400000

// Common float values in IEEE 754 format
#define F64_PLUS_ZERO      0x0000000000000000
#define F64_MINUS_ZERO     0x8000000000000000
#define F64_PLUS_ONE       0x3FF0000000000000
#define F64_MINUS_ONE      0xBFF0000000000000
#define F64_PLUS_INFINITE  0x7FF0000000000000
#define F64_MINUS_INFINITE 0xFFF0000000000000
#define F64_QUIET_NAN      0x7FF8000000000000
#define F64_SIGNALING_NAN  0x7FFC000000000000
#define F32_PLUS_ZERO      0x00000000
#define F32_MINUS_ZERO     0x80000000
#define F32_PLUS_ONE       0x3F800000
#define F32_MINUS_ONE      0xBF800000
#define F32_MINUS_INFINITE 0xFF800000
#define F32_PLUS_INFINITE  0x7F800000
#define F32_QUIET_NAN      0x7FC00000
#define F32_SIGNALING_NAN  0x7FE00000

// Negate a float by flipping its sign bit(s)
#define F64_NEGATE(x) ( (x) ^ F64_SIGN_BIT_MASK )
#define F32_NEGATE(x) ( (x) ^ F32_SIGN_BIT_MASK )

// Macro functions for extracting exponent, mantissa and checking for corner cases
#define F32_EXPONENT(a) ( ((a) & F32_EXPONENT_MASK) >> 23 )
#define F32_MANTISSA(a) ( (a) & F32_MANTISSA_MASK )

#define F32_IS_POSITIVE(a) ( ((a) & F32_SIGN_BIT_MASK) == 0 )
#define F32_IS_NEGATIVE(a) ( ((a) & F32_SIGN_BIT_MASK) != 0 )

#define F32_IS_ANY_INFINITY(a) ( (((a) & F32_EXPONENT_MASK) == F32_EXPONENT_MASK) && (((a) & F32_MANTISSA_MASK) == 0) )
#define F32_IS_PLUS_INFINITY(a) ( F32_IS_ANY_INFINITY(a) && F32_IS_POSITIVE(a) )
#define F32_IS_MINUS_INFINITY(a) ( F32_IS_ANY_INFINITY(a) && F32_IS_NEGATIVE(a) )

#define F32_IS_ANY_NAN(a) ( (((a) & F32_EXPONENT_MASK) == F32_EXPONENT_MASK) && (((a) & F32_MANTISSA_MASK) != 0) )
#define F32_IS_QUIET_NAN(a) ( (((a) & F32_EXPONENT_MASK) == F32_EXPONENT_MASK) && (((a) & F32_QUIET_NAN_MASK) != 0) )
#define F32_IS_SIGNALING_NAN(a) ( (((a) & F32_EXPONENT_MASK) == F32_EXPONENT_MASK) && (((a) & F32_MANTISSA_MASK) != 0) && (((a) & F32_QUIET_NAN_MASK) == 0) )

#define F32_IS_ANY_ZERO(a) ( (((a) & F32_EXPONENT_MASK) == 0) && (((a) & F32_MANTISSA_MASK) == 0) )
#define F32_IS_PLUS_ZERO(a) ( F32_IS_ANY_ZERO(a) && F32_IS_POSITIVE(a) )
#define F32_IS_MINUS_ZERO(a) ( F32_IS_ANY_ZERO(a) && F32_IS_NEGATIVE(a) )

#define F32_IS_NORMAL(a) ( ((a) & F32_EXPONENT_MASK) != 0 && ((a) & F32_EXPONENT_MASK) != F32_EXPONENT_MASK )
#define F32_IS_SUBNORMAL(a) ( ((a) & F32_EXPONENT_MASK) == 0 && ((a) & F32_MANTISSA_MASK) != 0 )

// Macro functions for extracting exponent, mantissa and checking for corner cases
#define F64_EXPONENT(a) ( ((a) & F64_EXPONENT_MASK) >> 52 )
#define F64_MANTISSA(a) ( (a) & F64_MANTISSA_MASK )

#define F64_IS_POSITIVE(a) ( ((a) & F64_SIGN_BIT_MASK) == 0 )
#define F64_IS_NEGATIVE(a) ( ((a) & F64_SIGN_BIT_MASK) != 0 )

#define F64_IS_ANY_INFINITY(a) ( (((a) & F64_EXPONENT_MASK) == F64_EXPONENT_MASK) && (((a) & F64_MANTISSA_MASK) == 0) )
#define F64_IS_PLUS_INFINITY(a) ( F64_IS_ANY_INFINITY(a) && F64_IS_POSITIVE(a) )
#define F64_IS_MINUS_INFINITY(a) ( F64_IS_ANY_INFINITY(a) && F64_IS_NEGATIVE(a) )

#define F64_IS_ANY_NAN(a) ( (((a) & F64_EXPONENT_MASK) == F64_EXPONENT_MASK) && (((a) & F64_MANTISSA_MASK) != 0) )
#define F64_IS_QUIET_NAN(a) ( (((a) & F64_EXPONENT_MASK) == F64_EXPONENT_MASK) && (((a) & F64_QUIET_NAN_MASK) != 0) )
#define F64_IS_SIGNALING_NAN(a) ( (((a) & F64_EXPONENT_MASK) == F64_EXPONENT_MASK) && (((a) & F64_MANTISSA_MASK) != 0) && (((a) & F64_QUIET_NAN_MASK) == 0) )

#define F64_IS_ANY_ZERO(a) ( (((a) & F64_EXPONENT_MASK) == 0) && (((a) & F64_MANTISSA_MASK) == 0) )
#define F64_IS_PLUS_ZERO(a) ( F64_IS_ANY_ZERO(a) && F64_IS_POSITIVE(a) )
#define F64_IS_MINUS_ZERO(a) ( F64_IS_ANY_ZERO(a) && F64_IS_NEGATIVE(a) )

#define F64_IS_NORMAL(a) ( ((a) & F64_EXPONENT_MASK) != 0 && ((a) & F64_EXPONENT_MASK) != F64_EXPONENT_MASK )
#define F64_IS_SUBNORMAL(a) ( ((a) & F64_EXPONENT_MASK) == 0 && ((a) & F64_MANTISSA_MASK) != 0 )

void _zisk_float (void);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
