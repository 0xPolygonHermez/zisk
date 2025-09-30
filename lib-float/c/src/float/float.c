//#include <stdint.h>
#include "softfloat.h"
#include "float.h"

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_ASSERT(condition) \
    do { \
        if (!(condition)) { \
            *(uint64_t *)0x0 = __LINE__; \
        } \
    } while (0)

void set_rounding_mode (uint64_t rm);
void update_rounding_mode (uint64_t * rm);
void change_rounding_mode_sign (void);

//extern uint64_t zisk_float_calls;

void _zisk_float (void)
{
    // Before calling any softfloat function, set the rounding mode from the fcsr register
    // into the softfloat_roundingMode variable.
    set_rounding_mode((fcsr >> 5) & 0x7);

    // Clear exception flags before operation
    softfloat_exceptionFlags = 0;

    uint64_t inst = *(uint64_t *)FREG_INST;
    switch (inst & 0x7F)
    {
        // The instructions flw/fld/fsw/fsd are handled in the main emulator loop, since they don't
        // require any floating-point operations; they just load/store from/to memory binary data.

        // case 7 : { // Opcode 7
        //     switch ((inst >> 12) & 0x7) {
        //         case 2: //("R", "flw"),
        //         case 3: //("R", "fld"),
        //         default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 7 inst=0x{inst:x}"),
        //     }
        // }

        // case 39 : // Opcode 39
        // {
        //     switch ((inst >> 12) & 0x7) {
        //         case 2: //("S", "fsw"),
        //         case 3: //("S", "fsd"),
        //         default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 39 inst=0x{inst:x}"),
        //     }
        // }

        case 67 : { // Opcode 67
            switch ((inst >> 25) & 0x3) {
                case 0: { //("R4", "fmadd.s"), rd = (rs1 x rs2) + rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;

                    // fmadd.s(∞, 0, 5.0) = NaN  # Invalid Operation! (∞ × 0 is undefined)
                    // fmadd.s(0, ∞, 5.0) = NaN  # Invalid Operation!
                    if ( (F32_IS_ANY_INFINITE(fregs[rs1]) && F32_IS_ANY_ZERO(fregs[rs2])) ||
                         (F32_IS_ANY_ZERO(fregs[rs1]) && F32_IS_ANY_INFINITE(fregs[rs2])) ) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // NaN propagation
                    if (F32_IS_NAN(fregs[rs1]) || F32_IS_NAN(fregs[rs2]) || F32_IS_NAN(fregs[rs3])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]) || F32_IS_SIGNALING_NAN(fregs[rs3]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    // fmadd.s(∞, 1, 5.0) = ∞    # Valid (∞ + 5.0 = ∞)
                    // fmadd.s(∞, 2, -∞) = NaN   # Invalid Operation! (∞ - ∞)
                    // fmadd.s(∞, 1, ∞) = ∞      # Valid (∞ + ∞ = ∞)
                    // fmadd.s(∞, -1, ∞) = NaN   # Invalid Operation! (-∞ + ∞)
                    if ( F32_IS_PLUS_INFINITE(fregs[rs1]) ) {
                        if ( F32_IS_POSITIVE(fregs[rs2]) ) {
                            if ( F32_IS_MINUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F32_PLUS_INFINITE;
                                break;
                            }
                        } else {
                            if ( F32_IS_PLUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F32_MINUS_INFINITE;
                                break;
                            }
                        }
                    }
                    if ( F32_IS_MINUS_INFINITE(fregs[rs1]) ) {
                        if ( F32_IS_POSITIVE(fregs[rs2]) ) {
                            if ( F32_IS_PLUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F32_MINUS_INFINITE;
                                break;
                            }
                        } else {
                            if ( F32_IS_MINUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F32_PLUS_INFINITE;
                                break;
                            }
                        }
                    }
                    if ( F32_IS_PLUS_INFINITE(fregs[rs2]) ) {
                        if ( F32_IS_POSITIVE(fregs[rs1]) ) {
                            if ( F32_IS_MINUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F32_PLUS_INFINITE;
                                break;
                            }
                        } else {
                            if ( F32_IS_PLUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F32_MINUS_INFINITE;
                                break;
                            }
                        }
                    }
                    if ( F32_IS_MINUS_INFINITE(fregs[rs2]) ) {
                        if ( F32_IS_POSITIVE(fregs[rs1]) ) {
                            if ( F32_IS_PLUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F32_MINUS_INFINITE;
                                break;
                            }
                        } else {
                            if ( F32_IS_MINUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F32_PLUS_INFINITE;
                                break;
                            }
                        }
                    }

                    // Get rounding mode
                    set_rounding_mode(rm);

                    // Call f32_mulAdd()
                    fregs[rd] = (uint64_t)f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v;
                    //softfloat_exceptionFlags &= ~softfloat_flag_underflow;
                    break;
                }
                case 1: { //=> ("R4", "fmadd.d"), rd = (rs1 x rs2) + rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;

                    // // infinity * NaN = NaN
                    // if (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_NAN(fregs[rs2])) {
                    //     fregs[rd] = F64_QUIET_NAN;
                    //     if (F64_IS_SIGNALING_NAN(fregs[rs2]))
                    //         softfloat_raiseFlags( softfloat_flag_invalid );
                    //     break;
                    // }
                    // // NaN * infinity = NaN
                    // if (F64_IS_NAN(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                    //     fregs[rd] = F64_QUIET_NAN;
                    //     if (F64_IS_SIGNALING_NAN(fregs[rs1]))
                    //         softfloat_raiseFlags( softfloat_flag_invalid );
                    //     break;
                    // }
                    // // NaN * NaN = NaN
                    // if (F64_IS_NAN(fregs[rs1]) || F64_IS_NAN(fregs[rs2])) {
                    //     fregs[rd] = F64_QUIET_NAN;
                    //     if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]))
                    //         softfloat_raiseFlags( softfloat_flag_invalid );
                    //     break;
                    // }
                    // // zero * infinity = NaN
                    // if (F64_IS_ANY_ZERO(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                    //     fregs[rd] = F64_QUIET_NAN;
                    //     softfloat_raiseFlags( softfloat_flag_invalid );
                    //     break;
                    // }
                    // // infinity * zero = NaN
                    // if (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_ANY_ZERO(fregs[rs2])) {
                    //     fregs[rd] = F64_QUIET_NAN;
                    //     softfloat_raiseFlags( softfloat_flag_invalid );
                    //     break;
                    // }

                    // fmadd.d(∞, 0, 5.0) = NaN  # Invalid Operation! (∞ × 0 is undefined)
                    // fmadd.d(0, ∞, 5.0) = NaN  # Invalid Operation!
                    if ( (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_ANY_ZERO(fregs[rs2])) ||
                         (F64_IS_ANY_ZERO(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) ) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // NaN propagation
                    if (F64_IS_NAN(fregs[rs1]) || F64_IS_NAN(fregs[rs2]) || F64_IS_NAN(fregs[rs3])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]) || F64_IS_SIGNALING_NAN(fregs[rs3]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // fmadd.d(∞, 1, 5.0) = ∞    # Valid (∞ + 5.0 = ∞)
                    // fmadd.d(∞, 2, -∞) = NaN   # Invalid Operation! (∞ - ∞)
                    // fmadd.d(∞, 1, ∞) = ∞      # Valid (∞ + ∞ = ∞)
                    // fmadd.d(∞, -1, ∞) = NaN   # Invalid Operation! (-∞ + ∞)
                    if ( F64_IS_PLUS_INFINITE(fregs[rs1]) ) {
                        if ( F64_IS_POSITIVE(fregs[rs2]) ) {
                            if ( F64_IS_MINUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F64_PLUS_INFINITE;
                                break;
                            }
                        } else {
                            if ( F64_IS_PLUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F64_MINUS_INFINITE;
                                break;  
                            }
                        }
                    }
                    if ( F64_IS_MINUS_INFINITE(fregs[rs1]) ) {
                        if ( F64_IS_POSITIVE(fregs[rs2]) ) {
                            if ( F64_IS_PLUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F64_MINUS_INFINITE;
                                break;
                            }
                        } else {
                            if ( F64_IS_MINUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F64_PLUS_INFINITE;
                                break;  
                            }
                        }
                    }
                    if ( F64_IS_PLUS_INFINITE(fregs[rs2]) ) {
                        if ( F64_IS_POSITIVE(fregs[rs1]) ) {
                            if ( F64_IS_MINUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F64_PLUS_INFINITE;
                                break;
                            }
                        } else {
                            if ( F64_IS_PLUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F64_MINUS_INFINITE;
                                break;  
                            }
                        }
                    }
                    if ( F64_IS_MINUS_INFINITE(fregs[rs2]) ) {
                        if ( F64_IS_POSITIVE(fregs[rs1]) ) {
                            if ( F64_IS_PLUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F64_MINUS_INFINITE;
                                break;
                            }
                        } else {
                            if ( F64_IS_MINUS_INFINITE(fregs[rs3]) ) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                                break;
                            } else {
                                fregs[rd] = F64_PLUS_INFINITE;
                                break;  
                            }
                        }
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f64_mulAdd()
                    fregs[rd] = (uint64_t)f64_mulAdd( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]}, (float64_t){fregs[rs3]} ).v;
                    break;
                }
                default: //_ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 67 inst=0x{inst:x}"),
                    FLOAT_ASSERT(false);
                    break;
            }
            break;
        }

        case 71 : { // Opcode 71
            switch ((inst >> 25) & 0x3) {
                case 0: { //("R4", "fmsub.s"), rd = (rs1 x rs2) - rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    
                    // NaN propagation
                    if (F32_IS_NAN(fregs[rs1]) || F32_IS_NAN(fregs[rs2]) || F32_IS_NAN(fregs[rs3])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]) || F32_IS_SIGNALING_NAN(fregs[rs3]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    
                    // fmsub.s(∞, 0, 5.0) = NaN  # Invalid Operation! (∞ × 0 is undefined)
                    // fmsub.s(0, ∞, 5.0) = NaN  # Invalid Operation!
                    if ( (F32_IS_ANY_INFINITE(fregs[rs1]) && F32_IS_ANY_ZERO(fregs[rs2])) ||
                         (F32_IS_ANY_ZERO(fregs[rs1]) && F32_IS_ANY_INFINITE(fregs[rs2])) ) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }

                    // Infinity multiplication and subtraction: +/-∞ - +/-∞
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) || F32_IS_ANY_INFINITE(fregs[rs2])) {
                        if (F32_IS_POSITIVE(fregs[rs1]) == F32_IS_POSITIVE(fregs[rs2])) { // rs1 and rs2 have the same sign, so multiplication is positive infinity
                            if (F32_IS_PLUS_INFINITE(fregs[rs3])) { // ∞ - ∞ = NaN
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // ∞ - -∞ = ∞
                                fregs[rd] = F32_PLUS_INFINITE;
                            }
                        } else { // rs1 and rs2 have different signs, so multiplication is negative infinity
                            if (F32_IS_MINUS_INFINITE(fregs[rs3])) { // -∞ - -∞ = NaN
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // -∞ - ∞ = -∞
                                fregs[rd] = F32_MINUS_INFINITE;
                            }
                        }
                        break;
                    }   

                    // Infinity subtraction
                    // fmsub.s(2.0, 3.0, ∞) = (2.0 × 3.0) - ∞ = 6.0 - ∞ = -∞
                    // fmsub.s(2.0, 3.0, -∞) = (2.0 × 3.0) - (-∞) = 6.0 + ∞ = +∞
                    if (!F32_IS_ANY_INFINITE(fregs[rs1]) && !F32_IS_ANY_INFINITE(fregs[rs2]) && F32_IS_ANY_INFINITE(fregs[rs3])) {
                        if (F32_IS_PLUS_INFINITE(fregs[rs3])) {
                            fregs[rd] = F32_MINUS_INFINITE;
                        } else {
                            fregs[rd] = F32_PLUS_INFINITE;
                        }
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f32_mulAdd()
                    fregs[rd] = (uint64_t)f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){NEG32(fregs[rs3])} ).v;
                    break;
                }
                case 1: { //=> ("R4", "fmsub.d"), rd = (rs1 x rs2) - rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    
                    // NaN propagation
                    if (F64_IS_NAN(fregs[rs1]) || F64_IS_NAN(fregs[rs2]) || F64_IS_NAN(fregs[rs3])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]) || F64_IS_SIGNALING_NAN(fregs[rs3]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // fmsub.d(∞, 0, 5.0) = NaN  # Invalid Operation! (∞ × 0 is undefined)
                    // fmsub.d(0, ∞, 5.0) = NaN  # Invalid Operation!
                    if ( (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_ANY_ZERO(fregs[rs2])) ||
                         (F64_IS_ANY_ZERO(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) ) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }

                    // Infinity multiplication and subtraction
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) || F64_IS_ANY_INFINITE(fregs[rs2])) {
                        if (F64_IS_POSITIVE(fregs[rs1]) == F64_IS_POSITIVE(fregs[rs2])) { // rs1 and rs2 have the same sign, so multiplication is positive infinity
                            if (F64_IS_PLUS_INFINITE(fregs[rs3])) { // ∞ - ∞ = NaN
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // ∞ - -∞ = ∞
                                fregs[rd] = F64_PLUS_INFINITE;
                            }
                        } else { // rs1 and rs2 have different signs, so multiplication is negative infinity
                            if (F64_IS_MINUS_INFINITE(fregs[rs3])) { // -∞ - -∞ = NaN
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // -∞ - ∞ = -∞
                                fregs[rd] = F64_MINUS_INFINITE;
                            }
                        }
                        break;
                    }   

                    // Infinity subtraction
                    // fmsub.d(2.0, 3.0, ∞) = (2.0 × 3.0) - ∞ = 6.0 - ∞ = -∞
                    // fmsub.d(2.0, 3.0, -∞) = (2.0 × 3.0) - (-∞) = 6.0 + ∞ = +∞
                    if (!F64_IS_ANY_INFINITE(fregs[rs1]) && !F64_IS_ANY_INFINITE(fregs[rs2]) && F64_IS_ANY_INFINITE(fregs[rs3])) {
                        if (F64_IS_PLUS_INFINITE(fregs[rs3])) {
                            fregs[rd] = F64_MINUS_INFINITE;
                        } else {
                            fregs[rd] = F64_PLUS_INFINITE;
                        }
                        break;
                    }

                    // Conflicting infinities (same sign)
                    // fmsub.d(∞, 1.0, ∞) = (∞ × 1.0) - ∞ = ∞ - ∞ = NaN (Invalid Operation)
                    // fmsub.d(-∞, 1.0, -∞) = (-∞ × 1.0) - (-∞) = -∞ + ∞ = NaN (Invalid Operation)

                    // Conflicting infinities (different signs)  
                    // fmsub.d(∞, 1.0, -∞) = (∞ × 1.0) - (-∞) = ∞ + ∞ = ∞
                    // fmsub.d(-∞, 1.0, ∞) = (-∞ × 1.0) - ∞ = -∞ - ∞ = -∞

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f64_mulAdd()
                    fregs[rd] = (uint64_t)f64_mulAdd( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]}, (float64_t){NEG64(fregs[rs3])} ).v;
                    break;
                }
                default: //_ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 71 inst=0x{inst:x}"),
                    FLOAT_ASSERT(false);
                    break;
            }
            break;
        }

        case 75 : { // Opcode 75
            switch ((inst >> 25) & 0x3) {
                case 0: { //("R4", "fnmsub.s"), rd = -(rs1 x rs2) + rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;

                    // sNaN propagation
                    if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]) || F32_IS_SIGNALING_NAN(fregs[rs3])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // infinity * zero = NaN
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) && F32_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // zero * infinity = NaN
                    if (F32_IS_ANY_ZERO(fregs[rs1]) && F32_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // qNaN propagation
                    if (F32_IS_QUIET_NAN(fregs[rs1]) || F32_IS_QUIET_NAN(fregs[rs2]) || F32_IS_QUIET_NAN(fregs[rs3])) {
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }

                    // Subtraction of something to infinity, i.e. multiplication of at least one infinity
                    // -(+∞ + +∞) = -∞
                    // -(+∞ + -∞) = NaN
                    // -(-∞ + +∞) = NaN
                    // -(-∞ + -∞) = +∞
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) || F32_IS_ANY_INFINITE(fregs[rs2])) { // Multiplication will result in infinity
                        if (F32_IS_POSITIVE(fregs[rs1]) == F32_IS_POSITIVE(fregs[rs2])) { // rs1 and rs2 have the same sign, so multiplication is positive infinity
                            if (F32_IS_PLUS_INFINITE(fregs[rs3])) { // -(+∞ - +∞) = NaN
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // -(+∞ - -∞ or x) = -∞
                                fregs[rd] = F32_MINUS_INFINITE;
                            }
                        } else { // rs1 and rs2 have different signs, so multiplication is negative infinity
                            if (F32_IS_MINUS_INFINITE(fregs[rs3])) { // -(-∞ - -∞) = NaN
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // -(-∞ - +∞ or x) = +∞
                                fregs[rd] = F32_PLUS_INFINITE;
                            }
                        }
                        break;
                    }

                    // Multiplication by zero
                    // -(0*rs2 - rs3) = -rs3, -(rs1*0 - rs3) = +rs3
                    if ((F32_IS_ANY_ZERO(fregs[rs1]) || F32_IS_ANY_ZERO(fregs[rs2])) && !F32_IS_ANY_ZERO(fregs[rs3])) {
                        fregs[rd] = fregs[rs3];
                        break;
                    }

                    // Addition of signed zeros
                    // +0 + +0 = +0
                    // +0 + -0 = +0
                    // -0 + +0 = +0
                    // -0 + -0 = -0
                    // if (F32_IS_ANY_ZERO(fregs[rs3])) {
                    //     if (F32_IS_ANY_ZERO(fregs[rs1]) || F32_IS_ANY_ZERO(fregs[rs2])) { // Multiplication is +/-0
                    //         if (F32_IS_POSITIVE(fregs[rs1]) != F32_IS_POSITIVE(fregs[rs2])) { // Multiplication is -0
                    //             if (F32_IS_POSITIVE(fregs[rs3])) {
                    //                 fregs[rd] = F32_PLUS_ZERO;
                    //             } else {
                    //                 fregs[rd] = F32_PLUS_ZERO;
                    //             }
                    //         } else { // Multiplication is +0
                    //             if (F32_IS_POSITIVE(fregs[rs3])) {
                    //                 fregs[rd] = F32_MINUS_ZERO;
                    //             } else {
                    //                 fregs[rd] = F32_PLUS_ZERO;
                    //             }
                    //         }
                    //         break;
                    //     }
                    // }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f32_mulAdd()
                    fregs[rd] = (uint64_t)f32_mulAdd( (float32_t){NEG32(fregs[rs1])}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v;

                    break;
                }
                case 1: { //=> ("R4", "fnmsub.d"), rd = -(rs1 x rs2) + rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;

                    // sNaN propagation
                    if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]) || F64_IS_SIGNALING_NAN(fregs[rs3])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // infinity * zero = NaN
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // zero * infinity = NaN
                    if (F64_IS_ANY_ZERO(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // qNaN propagation
                    if (F64_IS_QUIET_NAN(fregs[rs1]) || F64_IS_QUIET_NAN(fregs[rs2]) || F64_IS_QUIET_NAN(fregs[rs3])) {
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }

                    // Subtraction of something to infinity, i.e. multiplication of at least one infinity
                    // -(+∞ + +∞) = -∞
                    // -(+∞ + -∞) = NaN
                    // -(-∞ + +∞) = NaN
                    // -(-∞ + -∞) = +∞
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) || F64_IS_ANY_INFINITE(fregs[rs2])) { // Multiplication will result in infinity
                        if (F64_IS_POSITIVE(fregs[rs1]) == F64_IS_POSITIVE(fregs[rs2])) { // rs1 and rs2 have the same sign, so multiplication is positive infinity
                            if (F64_IS_PLUS_INFINITE(fregs[rs3])) { // -(+∞ - +∞) = NaN
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // -(+∞ - -∞ or x) = -∞
                                fregs[rd] = F64_MINUS_INFINITE;
                            }
                        } else { // rs1 and rs2 have different signs, so multiplication is negative infinity
                            if (F64_IS_MINUS_INFINITE(fregs[rs3])) { // -(-∞ - -∞) = NaN
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // -(-∞ - +∞ or x) = +∞
                                fregs[rd] = F64_PLUS_INFINITE;
                            }
                        }
                        break;
                    }

                    // Multiplication by zero
                    // -(0*rs2 - rs3) = -rs3, -(rs1*0 - rs3) = +rs3
                    if ((F64_IS_ANY_ZERO(fregs[rs1]) || F64_IS_ANY_ZERO(fregs[rs2])) && !F64_IS_ANY_ZERO(fregs[rs3])) {
                        fregs[rd] = fregs[rs3];
                        break;
                    }


                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f64_mulAdd()
                    fregs[rd] = (uint64_t)f64_mulAdd( (float64_t){NEG64(fregs[rs1])}, (float64_t){fregs[rs2]}, (float64_t){fregs[rs3]} ).v;
                    break;
                }
                default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 75 inst=0x{inst:x}"),
                    FLOAT_ASSERT(false);
                    break;
            }
            break;
        }

        case 79 : { // Opcode 79
            switch ((inst >> 25) & 0x3) {
                case 0: { //("R4", "fnmadd.s"), rd = -(rs1 x rs2) - rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;

                    // sNaN propagation
                    if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]) || F32_IS_SIGNALING_NAN(fregs[rs3])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // infinity * zero = NaN
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) && F32_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // zero * infinity = NaN
                    if (F32_IS_ANY_ZERO(fregs[rs1]) && F32_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // qNaN propagation
                    if (F32_IS_QUIET_NAN(fregs[rs1]) || F32_IS_QUIET_NAN(fregs[rs2]) || F32_IS_QUIET_NAN(fregs[rs3])) {
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }

                    // Subtraction of something to infinity, i.e. multiplication of at least one infinity
                    // -(+∞ + +∞) = -∞
                    // -(+∞ + -∞) = NaN
                    // -(-∞ + +∞) = NaN
                    // -(-∞ + -∞) = +∞
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) || F32_IS_ANY_INFINITE(fregs[rs2])) { // Multiplication will result in infinity
                        if (F32_IS_POSITIVE(fregs[rs1]) == F32_IS_POSITIVE(fregs[rs2])) { // rs1 and rs2 have the same sign, so multiplication is positive infinity
                            if (F32_IS_PLUS_INFINITE(fregs[rs3])) { // -(+∞ - +∞) = NaN
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // -(+∞ - -∞ or x) = -∞
                                fregs[rd] = F32_MINUS_INFINITE;
                            }
                        } else { // rs1 and rs2 have different signs, so multiplication is negative infinity
                            if (F32_IS_MINUS_INFINITE(fregs[rs3])) { // -(-∞ - -∞) = NaN
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_raiseFlags( softfloat_flag_invalid );
                            } else { // -(-∞ - +∞ or x) = +∞
                                fregs[rd] = F32_PLUS_INFINITE;
                            }
                        }
                        break;
                    }

                    // Multiplication by zero
                    // -(0*rs2 + rs3) = -rs3, -(rs1*0 + rs3) = -rs3
                    if ((F32_IS_ANY_ZERO(fregs[rs1]) || F32_IS_ANY_ZERO(fregs[rs2])) && !F32_IS_ANY_ZERO(fregs[rs3])) {
                        fregs[rd] = NEG32(fregs[rs3]);
                        break;
                    }

                    // Addition of signed zeros
                    // +0 + +0 = +0
                    // +0 + -0 = +0
                    // -0 + +0 = +0
                    // -0 + -0 = -0
                    if (F32_IS_ANY_ZERO(fregs[rs3])) {
                        if (F32_IS_ANY_ZERO(fregs[rs1]) || F32_IS_ANY_ZERO(fregs[rs2])) { // Multiplication is +/-0
                            if (F32_IS_POSITIVE(fregs[rs1]) != F32_IS_POSITIVE(fregs[rs2])) { // Multiplication is -0
                                if (F32_IS_POSITIVE(fregs[rs3])) {
                                    fregs[rd] = F32_PLUS_ZERO;
                                } else {
                                    fregs[rd] = F32_PLUS_ZERO;
                                }
                            } else { // Multiplication is +0
                                if (F32_IS_POSITIVE(fregs[rs3])) {
                                    fregs[rd] = F32_MINUS_ZERO;
                                } else {
                                    fregs[rd] = F32_PLUS_ZERO;
                                }
                            }                        
                            break;
                        }
                    }
                    // if (F32_IS_ANY_ZERO(fregs[rs1]) || F32_IS_ANY_ZERO(fregs[rs2])) {
                    //     if (F32_IS_ANY_ZERO(fregs[rs3])) {
                    //         if ((F32_IS_POSITIVE(fregs[rs1]) != F32_IS_POSITIVE(fregs[rs2])) && F32_IS_NEGATIVE(fregs[rs3])) {
                    //             fregs[rd] = F32_PLUS_ZERO;
                    //         } else {
                    //             fregs[rd] = F32_MINUS_ZERO;
                    //         }                        
                    //         break;
                    //     } else
                    //         fregs[rd] = NEG32(fregs[rs3]);
                    //     break;
                    // }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    change_rounding_mode_sign();

                    // Call f32_mulAdd()
                    uint32_t result = f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v;
                    if (result == F32_PLUS_ZERO)
                        fregs[rd] = (uint64_t)F32_MINUS_ZERO;
                    else if (result == F32_MINUS_ZERO)
                        fregs[rd] = (uint64_t)F32_PLUS_ZERO;
                    else
                        fregs[rd] = (uint64_t)NEG32(result);
                    // else
                    // fregs[rd] = (uint64_t)NEG32(f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v);
                    break;
                    // inst_12:
                    // // rs1==f8, rs2==f20, rs3==f3, rd==f30,fs1 == 0 and fe1 == 0x00 and fm1 == 0x000000 and fs2 == 0 and fe2 == 0x00 and fm2 == 0x000000 and fs3 == 1 and fe3 == 0x00 and fm3 == 0x000000 and  fcsr == 0x0 and rm_val == 7   
                    // /* opcode: fnmadd.s ; op1:f8; op2:f20; op3:f3; dest:f30; op1val:0x0; op2val:0x0;
                    // op3val:0x80000000; valaddr_reg:x3; val_offset:36*FLEN/8; rmval:dyn;
                    // testreg:x2; fcsr_val:0 */
                    // TEST_FPR4_OP(fnmadd.s, f30, f8, f20, f3, dyn, 0, 0, x3, 36*FLEN/8, x4, x1, x2)
                    // It results 80000000 instead of 00000000
                }
                case 1: { //=> ("R4", "fnmadd.d"), rd = -(rs1 x rs2) - rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;

                    // NaN propagation
                    if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]) || F64_IS_SIGNALING_NAN(fregs[rs3])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    if (F64_IS_QUIET_NAN(fregs[rs1]) || F64_IS_QUIET_NAN(fregs[rs2]) || F64_IS_QUIET_NAN(fregs[rs3])) {
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // infinity * zero = NaN
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // zero * infinity = NaN
                    if (F64_IS_ANY_ZERO(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }

                    // Multiplication by zero
                    // -(0*rs2 + rs3) = -rs3, -(rs1*0 + rs3) = -rs3
                    if ((F64_IS_ANY_ZERO(fregs[rs1]) || F64_IS_ANY_ZERO(fregs[rs2])) && !F64_IS_ANY_ZERO(fregs[rs3])) {
                        fregs[rd] = NEG64(fregs[rs3]);
                        break;
                    }

                    // Addition of signed zeros
                    // +0 + +0 = +0
                    // +0 + -0 = +0
                    // -0 + +0 = +0
                    // -0 + -0 = -0
                    if (F64_IS_ANY_ZERO(fregs[rs3])) {
                        if (F64_IS_ANY_ZERO(fregs[rs1]) || F64_IS_ANY_ZERO(fregs[rs2])) { // Multiplication is +/-0
                            if (F64_IS_POSITIVE(fregs[rs1]) != F64_IS_POSITIVE(fregs[rs2])) { // Multiplication is -0
                                if (F64_IS_POSITIVE(fregs[rs3])) {
                                    fregs[rd] = F64_PLUS_ZERO;
                                } else {
                                    fregs[rd] = F64_PLUS_ZERO;
                                }
                            } else { // Multiplication is +0
                                if (F64_IS_POSITIVE(fregs[rs3])) {
                                    fregs[rd] = F64_MINUS_ZERO;
                                } else {
                                    fregs[rd] = F64_PLUS_ZERO;
                                }
                            }                        
                            break;
                        }
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    change_rounding_mode_sign();

                    // Call f64_mulAdd()
                    fregs[rd] = (uint64_t)NEG64(f64_mulAdd( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]}, (float64_t){fregs[rs3]} ).v);
                    break;
                }
                default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 79 inst=0x{inst:x}"),
                    FLOAT_ASSERT(false);
                    break;
            }
            break;
        }

        case 83 : { // Opcode 83
            switch ((inst >> 25) & 0x7F) {
                case 0 : { //("R", "fadd.s"),
                    // Get registers
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;

                    // NaN propagation: x + NaN = NaN, NaN + x = NaN, NaN + NaN = NaN
                    if (F32_IS_NAN(fregs[rs1]) || F32_IS_NAN(fregs[rs2])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }

                    // fadd.s(∞, -∞) = NaN    # Invalid Operation! (opposite-signed infinity)
                    // fadd.s(∞, ∞) = ∞       # Valid operation
                    // fadd.s(-∞, -∞) = -∞    # Valid operation (opposite-signed infinity)
                    // fadd.s(-∞, ∞) = NaN    # Invalid Operation!
                    if (F32_IS_PLUS_INFINITE(fregs[rs1]) && F32_IS_PLUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_PLUS_INFINITE;
                        break;
                    }
                    if (F32_IS_PLUS_INFINITE(fregs[rs1]) && F32_IS_MINUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    if (F32_IS_MINUS_INFINITE(fregs[rs1]) && F32_IS_PLUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    if (F32_IS_MINUS_INFINITE(fregs[rs1]) && F32_IS_MINUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_MINUS_INFINITE;
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f32_add()
                    fregs[rd] = (uint64_t)f32_add( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 1 : { //("R", "fadd.d"),
                    // Get registers
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;

                    //if (zisk_float_calls == 0)
                    // {
                    //     fregs[rd] = zisk_float_calls;//0x12345678;
                    //     zisk_float_calls++;
                    //     break;
                    // }
                    // zisk_float_calls++;

                    // NaN propagation: x + NaN = NaN, NaN + x = NaN, NaN + NaN = NaN
                    if (F64_IS_NAN(fregs[rs1]) || F64_IS_NAN(fregs[rs2])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // fadd.d(∞, -∞) = NaN    # Invalid Operation! (opposite-signed infinity)
                    // fadd.d(∞, ∞) = ∞       # Valid operation
                    // fadd.d(-∞, -∞) = -∞    # Valid operation (opposite-signed infinity)
                    // fadd.d(-∞, ∞) = NaN    # Invalid Operation!
                    if (F64_IS_PLUS_INFINITE(fregs[rs1]) && F64_IS_PLUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_PLUS_INFINITE;
                        break;
                    }
                    if (F64_IS_PLUS_INFINITE(fregs[rs1]) && F64_IS_MINUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    if (F64_IS_MINUS_INFINITE(fregs[rs1]) && F64_IS_PLUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    if (F64_IS_MINUS_INFINITE(fregs[rs1]) && F64_IS_MINUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_MINUS_INFINITE;
                        break;
                    }
                    // +0 + -0 = +0
                    if (F64_IS_PLUS_ZERO(fregs[rs1]) && F64_IS_MINUS_ZERO(fregs[rs2])) {
                        fregs[rd] = F64_PLUS_ZERO;
                        break;
                    }
                    // 0 + x = x
                    if (F64_IS_ANY_ZERO(fregs[rs1])) {
                        fregs[rd] = fregs[rs2];
                        break;
                    }
                    // x + 0 = x
                    if (F64_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = fregs[rs1];
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f64_add()
                    fregs[rd] = (uint64_t)f64_add( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 4 : { //("R", "fsub.s"),
                    // Get registers
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;

                    // NaN propagation: x - NaN = NaN, NaN - x = NaN, NaN - NaN = NaN
                    if (F32_IS_NAN(fregs[rs1]) || F32_IS_NAN(fregs[rs2])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    // fsub.s(∞, ∞) = NaN    # Invalid Operation! (same-signed infinity)
                    // fsub.s(∞, -∞) = ∞     # Valid operation
                    // fsub.s(-∞, ∞) = -∞    # Valid operation
                    // fsub.s(-∞, -∞) = NaN  # Invalid Operation! (same-signed infinity)
                    if (F32_IS_PLUS_INFINITE(fregs[rs1]) && F32_IS_PLUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    if (F32_IS_PLUS_INFINITE(fregs[rs1]) && F32_IS_MINUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_PLUS_INFINITE;
                        break;
                    }
                    if (F32_IS_MINUS_INFINITE(fregs[rs1]) && F32_IS_PLUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_MINUS_INFINITE;
                        break;
                    }
                    if (F32_IS_MINUS_INFINITE(fregs[rs1]) && F32_IS_MINUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f32_sub()
                    fregs[rd] = (uint64_t)f32_sub( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    // if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                    //     //if (fregs[rs1] == 0)
                    //         fregs[rd] = 0x7fc00000;
                    // };
                    break;
                }
                case 5 : { //("R", "fsub.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    
                    // NaN propagation
                    if (F64_IS_NAN(fregs[rs1]) || F64_IS_NAN(fregs[rs2])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // -∞ - (-∞) → Invalid Operation → NaN
                    if (F64_IS_MINUS_INFINITE(fregs[rs1]) && F64_IS_MINUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // ∞ - ∞ → Invalid Operation → NaN
                    if (F64_IS_PLUS_INFINITE(fregs[rs1]) && F64_IS_PLUS_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // ∞ - finite → ∞ (same sign as first ∞)
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) && !F64_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = fregs[rs1];
                        break;
                    }
                    // finite - ∞ → ∞ (opposite sign of second ∞)
                    if (!F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = NEG64(fregs[rs2]);
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f64_sub()
                    fregs[rd] = (uint64_t)f64_sub( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 8 : { //("R", "fmul.s"),
                    // Get registers
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;

                    // infinity * NaN = NaN
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) && F32_IS_NAN(fregs[rs2])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    // NaN * infinity = NaN
                    if (F32_IS_NAN(fregs[rs1]) && F32_IS_ANY_INFINITE(fregs[rs2])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs1]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    // NaN * NaN = NaN
                    if (F32_IS_NAN(fregs[rs1]) || F32_IS_NAN(fregs[rs2])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    // zero * infinity = NaN
                    if (F32_IS_ANY_ZERO(fregs[rs1]) && F32_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // infinity * zero = NaN
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) && F32_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f32_mul()
                    fregs[rd] = (uint64_t)f32_mul( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    if ((softfloat_exceptionFlags & softfloat_flag_underflow) && ((fregs[rd] & F32_SIGN_BIT_MASK) == 0) && ((fregs[rd] & F32_EXPONENT_MASK) != 0)) {
                        softfloat_exceptionFlags &= ~softfloat_flag_underflow;
                    }
                    break;
                }
                case 9 : { //("R", "fmul.d"),
                    // Get registers
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;

                    // infinity * NaN = NaN
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_NAN(fregs[rs2])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // NaN * infinity = NaN
                    if (F64_IS_NAN(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs1]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // NaN * NaN = NaN
                    if (F64_IS_NAN(fregs[rs1]) || F64_IS_NAN(fregs[rs2])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // zero * infinity = NaN
                    if (F64_IS_ANY_ZERO(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // infinity * zero = NaN
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f64_mul()
                    //bool both_non_zero = !F64_IS_ANY_ZERO(fregs[rs1]) && !F64_IS_ANY_ZERO(fregs[rs2]);
                    fregs[rd] = (uint64_t)f64_mul( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    // if (both_non_zero && F64_IS_ANY_ZERO(fregs[rd])) {
                    //     // It was an underflow, but we got a zero result. Clear the underflow flag.
                    //     softfloat_exceptionFlags |= softfloat_flag_underflow;
                    //     softfloat_exceptionFlags |= softfloat_flag_inexact;
                    //     softfloat_exceptionFlags = 7;
                    //     fregs[rd] = 0x12345678;
                    // }
                    break;
                    // riscv-arch-test/riscv-test-suite/rv32i_m/D/src/fmul.d_b9-01.S

                    // inst_1263:
                    // // fs1 == 1 and fe1 == 0x000 and fm1 == 0x0000000000001 and fs2 == 1 and fe2 == 0x37c and fm2 == 0x9999999999998 and  fcsr == 0x0 and rm_val == 7   
                    // /* opcode: fmul.d ; op1:f30; op2:f29; dest:f31; op1val:0x8000000000000001; op2val:0xb7c9999999999998;
                    //    valaddr_reg:x3; val_offset:2526*FLEN/8; rmval:dyn; fcsr: 0;
                    //    correctval:??; testreg:x2
                    // */
                    // TEST_FPRR_OP(fmul.d, f31, f30, f29, dyn, 0, 0, x3, 2526*FLEN/8, x4, x1, x2)
                }
                case 12 : { //("R", "fdiv.s"),
                    // Get registers
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;

                    // zero / zero = NaN
                    if (F32_IS_ANY_ZERO(fregs[rs1]) && F32_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // infinity / NaN = NaN
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) && F32_IS_NAN(fregs[rs2])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    // NaN / infinity = NaN
                    if (F32_IS_NAN(fregs[rs1]) && F32_IS_ANY_INFINITE(fregs[rs2])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs1]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    // NaN / NaN = NaN
                    if (F32_IS_NAN(fregs[rs1]) || F32_IS_NAN(fregs[rs2])) {
                        if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F32_QUIET_NAN;
                        break;
                    }
                    // infinity / infinity = NaN
                    if (F32_IS_ANY_INFINITE(fregs[rs1]) && F32_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = F32_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f64_div()
                    fregs[rd] = (uint64_t)f32_div( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 13 : { //("R", "fdiv.d"),
                    // Get registers
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;

                    // zero / zero = NaN
                    if (F64_IS_ANY_ZERO(fregs[rs1]) && F64_IS_ANY_ZERO(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }
                    // infinity / NaN = NaN
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_NAN(fregs[rs2])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // NaN / infinity = NaN
                    if (F64_IS_NAN(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs1]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // NaN / NaN = NaN
                    if (F64_IS_NAN(fregs[rs1]) || F64_IS_NAN(fregs[rs2])) {
                        if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]))
                            softfloat_raiseFlags( softfloat_flag_invalid );
                        fregs[rd] = F64_QUIET_NAN;
                        break;
                    }
                    // infinity / infinity = NaN
                    if (F64_IS_ANY_INFINITE(fregs[rs1]) && F64_IS_ANY_INFINITE(fregs[rs2])) {
                        fregs[rd] = F64_QUIET_NAN;
                        softfloat_raiseFlags( softfloat_flag_invalid );
                        break;
                    }

                    // Get rounding mode
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);

                    // Call f64_div()
                    fregs[rd] = (uint64_t)f64_div( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 16 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : { //("R", "fsgnj.s"), takes sign bit of rs2 and copies rs1 to rd
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            if (fregs[rs2] & F32_SIGN_BIT_MASK)
                                fregs[rd] = fregs[rs1] | F32_SIGN_BIT_MASK;
                            else
                                fregs[rd] = fregs[rs1] & (~F32_SIGN_BIT_MASK);
                            break;
                        }
                        case 1 : { //("R", "fsgnjn.s"), negates sign bit of rs2 and copies rs1 to rd
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            if (fregs[rs2] & F32_SIGN_BIT_MASK)
                                fregs[rd] = fregs[rs1] & (~F32_SIGN_BIT_MASK);
                            else
                                fregs[rd] = fregs[rs1] | F32_SIGN_BIT_MASK;
                            break;
                        }
                        case 2 : { //("R", "fsgnjx.s"), XORs sign bits of rs1 and rs2 and copies rs1 to rd
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            if (fregs[rs2] & F32_SIGN_BIT_MASK)
                                fregs[rd] = fregs[rs1] ^ F32_SIGN_BIT_MASK;
                            else
                                fregs[rd] = fregs[rs1];
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=16 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 17 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : { //("R", "fsgnj.d"), takes sign bit of rs2 and copies rs1 to rd
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            if (fregs[rs2] & F64_SIGN_BIT_MASK)
                                fregs[rd] = fregs[rs1] | F64_SIGN_BIT_MASK;
                            else
                                fregs[rd] = fregs[rs1] & (~F64_SIGN_BIT_MASK);
                            break;
                        }
                        case 1 : { //("R", "fsgnjn.d"), negates sign bit of rs2 and copies rs1 to rd
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            if (fregs[rs2] & F64_SIGN_BIT_MASK)
                                fregs[rd] = fregs[rs1] & (~F64_SIGN_BIT_MASK);
                            else
                                fregs[rd] = fregs[rs1] | F64_SIGN_BIT_MASK;
                            break;
                        }
                        case 2 : { //("R", "fsgnjx.d"), XORs sign bits of rs1 and rs2 and copies rs1 to rd
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            if (fregs[rs2] & F64_SIGN_BIT_MASK)
                                fregs[rd] = fregs[rs1] ^ F64_SIGN_BIT_MASK;
                            else
                                fregs[rd] = fregs[rs1];
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=17 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 20 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : { //("R", "fmin.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;

                            // fmax(+0.0, -0.0) = +0.0
                            if (F32_IS_PLUS_ZERO(fregs[rs1]) && F32_IS_MINUS_ZERO(fregs[rs2])) {
                                fregs[rd] = F32_MINUS_ZERO;
                                break;
                            }
                            // fmax(-0.0, +0.0) = +0.0
                            if (F32_IS_MINUS_ZERO(fregs[rs1]) && F32_IS_PLUS_ZERO(fregs[rs2])) {
                                fregs[rd] = F32_MINUS_ZERO;
                                break;
                            }
                            // fmax(NaN, NaN) = NaN
                            if (F32_IS_NAN(fregs[rs1]) && F32_IS_NAN(fregs[rs2])) {
                                if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = F32_QUIET_NAN;
                                break;
                            }
                            // fmax(x, NaN) = x
                            if (F32_IS_NAN(fregs[rs1])) {
                                if (F32_IS_SIGNALING_NAN(fregs[rs1]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = fregs[rs2];
                                break;
                            }
                            // fmax(NaN, x) = x
                            if (F32_IS_NAN(fregs[rs2])) {
                                if (F32_IS_SIGNALING_NAN(fregs[rs2]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = fregs[rs1];
                                break;
                            }

                            // Call f32_lt()
                            fregs[rd] = f32_lt( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? fregs[rs1] : fregs[rs2];
                            break;
                        }
                        case 1 : { //("R", "fmax.s"),
                            // The value -0.0 is considered to be less than the value +0.0. If both inputs are NaNs, the result is the
                            // canonical NaN. If only one operand is a NaN, the result is the non-NaN operand. Signaling NaN inputs
                            // set the invalid operation exception flag, even when the result is not NaN.
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;

                            // fmax(+0.0, -0.0) = +0.0
                            if (F32_IS_PLUS_ZERO(fregs[rs1]) && F32_IS_MINUS_ZERO(fregs[rs2])) {
                                fregs[rd] = F32_PLUS_ZERO;
                                break;
                            }
                            // fmax(-0.0, +0.0) = +0.0
                            if (F32_IS_MINUS_ZERO(fregs[rs1]) && F32_IS_PLUS_ZERO(fregs[rs2])) {
                                fregs[rd] = F32_PLUS_ZERO;
                                break;
                            }
                            // fmax(NaN, NaN) = NaN
                            if (F32_IS_NAN(fregs[rs1]) && F32_IS_NAN(fregs[rs2])) {
                                if (F32_IS_SIGNALING_NAN(fregs[rs1]) || F32_IS_SIGNALING_NAN(fregs[rs2]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = F32_QUIET_NAN;
                                break;
                            }
                            // fmax(x, NaN) = x
                            if (F32_IS_NAN(fregs[rs1])) {
                                if (F32_IS_SIGNALING_NAN(fregs[rs1]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = fregs[rs2];
                                break;
                            }
                            // fmax(NaN, x) = x
                            if (F32_IS_NAN(fregs[rs2])) {
                                if (F32_IS_SIGNALING_NAN(fregs[rs2]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = fregs[rs1];
                                break;
                            }

                            // Call f32_lt()
                            fregs[rd] = f32_lt( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? fregs[rs2] : fregs[rs1];
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=20 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 21 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : { //("R", "fmin.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            if (F64_IS_NAN(fregs[rs1]) && F64_IS_NAN(fregs[rs2])) {
                                if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = F64_QUIET_NAN;
                                break;
                            }
                            if (F64_IS_NAN(fregs[rs1])) {
                                if (F64_IS_SIGNALING_NAN(fregs[rs1]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = fregs[rs2];
                                break;
                            }
                            if (F64_IS_NAN(fregs[rs2])) {
                                if (F64_IS_SIGNALING_NAN(fregs[rs2]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = fregs[rs1];
                                break;
                            }
                            if (fregs[rs1] == F64_MINUS_ZERO && fregs[rs2] == F64_PLUS_ZERO) {
                                fregs[rd] = F64_MINUS_ZERO;
                                break;
                            }
                            if (fregs[rs1] == F64_PLUS_ZERO && fregs[rs2] == F64_MINUS_ZERO) {
                                fregs[rd] = F64_MINUS_ZERO;
                                break;
                            }
                            fregs[rd] = f64_lt( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? fregs[rs1] : fregs[rs2];
                            break;
                        }
                        case 1 : { //("R", "fmax.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            if (F64_IS_NAN(fregs[rs1]) && F64_IS_NAN(fregs[rs2])) {
                                if (F64_IS_SIGNALING_NAN(fregs[rs1]) || F64_IS_SIGNALING_NAN(fregs[rs2]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = F64_QUIET_NAN;
                                break;
                            }
                            if (F64_IS_NAN(fregs[rs1])) {
                                if (F64_IS_SIGNALING_NAN(fregs[rs1]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = fregs[rs2];
                                break;
                            }
                            if (F64_IS_NAN(fregs[rs2])) {
                                if (F64_IS_SIGNALING_NAN(fregs[rs2]))
                                    softfloat_exceptionFlags |= softfloat_flag_invalid;
                                fregs[rd] = fregs[rs1];
                                break;
                            }
                            if (fregs[rs1] == F64_MINUS_ZERO && fregs[rs2] == F64_PLUS_ZERO) {
                                fregs[rd] = F64_PLUS_ZERO;
                                break;
                            }
                            if (fregs[rs1] == F64_PLUS_ZERO && fregs[rs2] == F64_MINUS_ZERO) {
                                fregs[rd] = F64_PLUS_ZERO;
                                break;
                            }
                            fregs[rd] = f64_lt( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? fregs[rs2] : fregs[rs1];
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=21 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 32 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 1 : { //("R", "fcvt.s.d"),
                            // Get registers
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;

                            // Get value
                            if (F64_IS_QUIET_NAN(fregs[rs1])) {
                                fregs[rd] = F32_QUIET_NAN;
                            }
                            else if (F64_IS_SIGNALING_NAN(fregs[rs1])) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_exceptionFlags |= softfloat_flag_invalid;
                            } else {
                                // Get rounding mode
                                uint64_t rm = (inst >> 12) & 0x7;
                                set_rounding_mode(rm);

                                // Call f64_to_f32()
                                fregs[rd] = (uint64_t)f64_to_f32( (float64_t){fregs[rs1]} ).v;
                                if (F32_IS_QUIET_NAN(fregs[rd])) {
                                    softfloat_exceptionFlags &= ~softfloat_flag_invalid;
                                }
                            }

                            // Extend to 64 bits
                            fregs[rd] |= 0xFFFFFFFF00000000;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=32 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 33 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fcvt.d.s"),
                            // Get registers
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;

                            // Filter out invalid values
                            if (fregs[rs1] & 0xFFFFFFFF00000000) {
                                fregs[rd] = F64_QUIET_NAN;
                                break;
                            }
                            if (F32_IS_SIGNALING_NAN(fregs[rs1])) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_exceptionFlags |= softfloat_flag_invalid;
                                break;
                            }
                            if (F32_IS_QUIET_NAN(fregs[rs1])) {
                                fregs[rd] = F64_QUIET_NAN;
                                break;
                            }
                            if (F32_IS_PLUS_INFINITE(fregs[rs1])) {
                                fregs[rd] = F64_PLUS_INFINITE;
                                break;
                            }
                            if (F32_IS_MINUS_INFINITE(fregs[rs1])) {
                                fregs[rd] = F64_MINUS_INFINITE;
                                break;
                            }
                            if (F32_IS_PLUS_ZERO(fregs[rs1])) {
                                fregs[rd] = F64_QUIET_NAN;
                                break;
                            }
                            if (F32_IS_MINUS_ZERO(fregs[rs1])) {
                                fregs[rd] = F64_QUIET_NAN;
                                break;
                            }
                            if (F32_IS_SUBNORMAL(fregs[rs1])) {
                                fregs[rd] = F64_QUIET_NAN;
                                break;
                            }

                            // Get rounding mode
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);

                            // Call f32_to_f64()
                            fregs[rd] = (uint64_t)f32_to_f64( (float32_t){fregs[rs1]} ).v;

                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=33 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 44 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fsqrt.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            if (F32_IS_PLUS_INFINITE(fregs[rs1])) {
                                fregs[rd] = F32_PLUS_INFINITE;
                                break;
                            }
                            if (F32_IS_QUIET_NAN(fregs[rs1])) {
                                fregs[rd] = F32_QUIET_NAN;
                                break;
                            }
                            if (F32_IS_SIGNALING_NAN(fregs[rs1])) {
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_exceptionFlags |= softfloat_flag_invalid;
                                break;
                            }
                            if (F32_IS_MINUS_ZERO(fregs[rs1])) {
                                fregs[rd] = fregs[rs1];
                                break;
                            }
                            if (F32_IS_NEGATIVE(fregs[rs1])) {
                                // square root of negative number = NaN
                                fregs[rd] = F32_QUIET_NAN;
                                softfloat_exceptionFlags |= softfloat_flag_invalid;
                                break;
                            }
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)f32_sqrt( (float32_t){fregs[rs1]} ).v;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=44 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 45 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fsqrt.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            if (F64_IS_PLUS_INFINITE(fregs[rs1])) {
                                fregs[rd] = F64_PLUS_INFINITE;
                                break;
                            }
                            if (F64_IS_QUIET_NAN(fregs[rs1])) {
                                fregs[rd] = F64_QUIET_NAN;
                                break;
                            }
                            if (F64_IS_SIGNALING_NAN(fregs[rs1])) {
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_exceptionFlags |= softfloat_flag_invalid;
                                break;
                            }
                            if (F64_IS_MINUS_ZERO(fregs[rs1])) {
                                fregs[rd] = fregs[rs1];
                                break;
                            }
                            if (F64_IS_NEGATIVE(fregs[rs1])) {
                                // square root of negative number = NaN
                                fregs[rd] = F64_QUIET_NAN;
                                softfloat_exceptionFlags |= softfloat_flag_invalid;
                                break;
                            }
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)f64_sqrt( (float64_t){fregs[rs1]} ).v;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=45 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 80 : {
                    switch ((inst >> 12) & 0x7) {
                        case 2 : { //("R", "feq.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs_x[rd] = f32_eq( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? 1 : 0;
                            break;
                        }
                        case 1 : { //("R", "flt.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs_x[rd] = f32_lt( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? 1 : 0;
                            break;
                        }
                        case 0 : { //("R", "fle.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs_x[rd] = f32_le( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? 1 : 0;
                            break;
                        }
                        default: // => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=80 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 81 : {
                    switch ((inst >> 12) & 0x7) {
                        case 2 : { //("R", "feq.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs_x[rd] = f64_eq( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? 1 : 0;
                            break;
                        }
                        case 1 : { //("R", "flt.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs_x[rd] = f64_lt( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? 1 : 0;
                            break;
                        }
                        case 0 : { //("R", "fle.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs_x[rd] = f64_le( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? 1 : 0;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=81 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 96: {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fcvt.w.s"), converts float(rs1) to int32_t(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            update_rounding_mode(&rm);
                            fregs_x[rd] = (uint64_t)f32_to_i32( (float32_t){fregs[rs1]}, rm, true );

                            // If the instruction was invalid, i.e. the input is NaN or the
                            // conversion is out of range, we need to set the output according to
                            // the RISC-V spec. See section 20.7, table 28.
                            if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                                // If input is NaN, output is all 1's
                                if (F32_IS_NAN(fregs[rs1]))
                                    fregs_x[rd] = 0x7FFFFFFF;
                                // If input is negative and out of range, output is 0
                                else if (fregs[rs1] & F32_SIGN_BIT_MASK)
                                    fregs_x[rd] = 0xFFFFFFFF80000000;
                                // If input is positive and out of range, output is all 1's
                                else
                                    fregs_x[rd] = 0x7FFFFFFF;
                            }
                            // If result is negative, sign extend to 64 bits
                            else if (fregs_x[rd] & F32_SIGN_BIT_MASK)
                                fregs_x[rd] |= 0xFFFFFFFF00000000;

                            break;
                        }
                        case 1 : { //("R", "fcvt.wu.s"), converts float(rs1) to uint32_t(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            update_rounding_mode(&rm);
                            fregs_x[rd] = (uint64_t)f32_to_ui32( (float32_t){fregs[rs1]}, rm, true );

                            // If the instruction was invalid, i.e. the input is NaN or the
                            // conversion is out of range, we need to set the output according to
                            // the RISC-V spec. See section 20.7, table 28.
                            if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                                // If input is NaN, output is all 1's
                                if (F32_IS_NAN(fregs[rs1]))
                                    fregs_x[rd] = 0xFFFFFFFFFFFFFFFF;
                                // If input is negative and out of range, output is 0
                                else if (fregs[rs1] & F32_SIGN_BIT_MASK)
                                    fregs_x[rd] = 0;
                                // If input is positive and out of range, output is all 1's
                                else
                                    fregs_x[rd] = 0xFFFFFFFFFFFFFFFF;
                            }
                            // If result is negative, sign extend to 64 bits
                            else if (fregs_x[rd] & F32_SIGN_BIT_MASK)
                                fregs_x[rd] |= 0xFFFFFFFF00000000;

                            break;
                        }
                        case 2 : { //("R", "fcvt.l.s"), converts float(rs1) to int64_t(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            update_rounding_mode(&rm);
                            fregs_x[rd] = (uint64_t)f32_to_i64( (float32_t){fregs[rs1]}, rm, true );

                            // If the instruction was invalid, i.e. the input is NaN or the
                            // conversion is out of range, we need to set the output according to
                            // the RISC-V spec. See section 20.7, table 28.
                            if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                                // If input is NaN, output is all 1's
                                if (F32_IS_NAN(fregs[rs1]))
                                    fregs_x[rd] = 0x7FFFFFFFFFFFFFFF;
                                // If input is negative and out of range, output is all 1's
                                else if (fregs[rs1] & F32_SIGN_BIT_MASK)
                                    fregs_x[rd] = 0x8000000000000000;
                                // If input is positive and out of range, output is all 1's
                                else
                                    fregs_x[rd] = 0x7FFFFFFFFFFFFFFF;
                            }
                            
                            break;
                        }
                        case 3 : { //("R", "fcvt.lu.s"), converts float(rs1) to uint64_t(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            update_rounding_mode(&rm);
                            fregs_x[rd] = (uint64_t)f32_to_ui64( (float32_t){fregs[rs1]}, rm, true );

                            // If the instruction was invalid, i.e. the input is NaN or the
                            // conversion is out of range, we need to set the output according to
                            // the RISC-V spec. See section 20.7, table 28.
                            if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                                // If input is NaN, output is all 1's
                                if (F32_IS_NAN(fregs[rs1]))
                                    fregs_x[rd] = 0xFFFFFFFFFFFFFFFF;
                                // If input is negative and out of range, output is 0
                                else if (fregs[rs1] & F32_SIGN_BIT_MASK)
                                    fregs_x[rd] = 0;
                                // If input is positive and out of range, output is all 1's
                                else
                                    fregs_x[rd] = 0xFFFFFFFFFFFFFFFF;
                            }

                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=96 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 97 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fcvt.w.d"), converts double(rs1) to int32_t(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            update_rounding_mode(&rm);
                            fregs_x[rd] = (uint64_t)f64_to_i32( (float64_t){fregs[rs1]}, rm, true );

                            // If the instruction was invalid, i.e. the input is NaN or the
                            // conversion is out of range, we need to set the output according to
                            // the RISC-V spec. See section 20.7, table 28.
                            if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                                // If input is NaN, output is all 1's
                                if (F64_IS_NAN(fregs[rs1]))
                                    fregs_x[rd] = 0x7FFFFFFF;
                                // If input is negative and out of range, output is 0
                                else if (fregs[rs1] & F64_SIGN_BIT_MASK)
                                    fregs_x[rd] = 0xFFFFFFFF80000000;
                                // If input is positive and out of range, output is all 1's
                                else
                                    fregs_x[rd] = 0x7FFFFFFF;
                            }
                            // If result is negative, sign extend to 64 bits
                            else if (fregs_x[rd] & F32_SIGN_BIT_MASK)
                                fregs_x[rd] |= 0xFFFFFFFF00000000;

                            break;
                        }
                        case 1 : { //("R", "fcvt.wu.d"), converts double(rs1) to uint32_t(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            update_rounding_mode(&rm);
                            fregs_x[rd] = (uint64_t)f64_to_ui32( (float64_t){fregs[rs1]}, rm, true );

                            // If the instruction was invalid, i.e. the input is NaN or the
                            // conversion is out of range, we need to set the output according to
                            // the RISC-V spec. See section 20.7, table 28.
                            if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                                // If input is NaN, output is all 1's
                                if (F64_IS_NAN(fregs[rs1]))
                                    fregs_x[rd] = 0xFFFFFFFFFFFFFFFF;
                                // If input is negative and out of range, output is 0
                                else if (fregs[rs1] & F64_SIGN_BIT_MASK)
                                    fregs_x[rd] = 0;
                                // If input is positive and out of range, output is all 1's
                                else
                                    fregs_x[rd] = 0xFFFFFFFFFFFFFFFF;
                            }
                            // If result is negative, sign extend to 64 bits
                            else if (fregs_x[rd] & F32_SIGN_BIT_MASK)
                                fregs_x[rd] |= 0xFFFFFFFF00000000;

                            break;
                        }
                        case 2 : { //("R", "fcvt.l.d"), converts double(rs1) to int64_t(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            update_rounding_mode(&rm);
                            fregs_x[rd] = (int64_t)f64_to_i64( (float64_t){fregs[rs1]}, rm, true );

                            // If the instruction was invalid, i.e. the input is NaN or the
                            // conversion is out of range, we need to set the output according to
                            // the RISC-V spec. See section 20.7, table 28.
                            if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                                // If input is NaN, output is all 1's
                                if (F64_IS_NAN(fregs[rs1]))
                                    fregs_x[rd] = 0x7FFFFFFFFFFFFFFF;
                                // If input is negative and out of range, output is all 1's
                                else if (fregs[rs1] & F64_SIGN_BIT_MASK)
                                    fregs_x[rd] = 0x8000000000000000;
                                // If input is positive and out of range, output is all 1's
                                else
                                    fregs_x[rd] = 0x7FFFFFFFFFFFFFFF;
                            }

                            break;
                        }
                        case 3 : { //("R", "fcvt.lu.d"), converts double(rs1) to uint64_t(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            update_rounding_mode(&rm);
                            fregs_x[rd] = f64_to_ui64( (float64_t){fregs[rs1]}, rm, true );

                            // If the instruction was invalid, i.e. the input is NaN or the
                            // conversion is out of range, we need to set the output according to
                            // the RISC-V spec. See section 20.7, table 28.
                            if (softfloat_exceptionFlags & softfloat_flag_invalid) {
                                // If input is NaN, output is all 1's
                                if (F64_IS_NAN(fregs[rs1]))
                                    fregs_x[rd] = 0xFFFFFFFFFFFFFFFF;
                                // If input is negative and out of range, output is 0
                                else if (fregs[rs1] & F64_SIGN_BIT_MASK)
                                    fregs_x[rd] = 0;
                                // If input is positive and out of range, output is all 1's
                                else
                                    fregs_x[rd] = 0xFFFFFFFFFFFFFFFF;
                            }

                            break;
                        }
                        default: // => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=97 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 104 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fcvt.s.w"), converts int32_t(rs1) to float(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)i32_to_f32( (int32_t)(fregs_x[rs1]) ).v;
                            break;
                        }
                        case 1 : { //("R", "fcvt.s.wu"), converts uint32_t(rs1) to float(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)ui32_to_f32( (uint32_t)(fregs_x[rs1]) ).v;
                            break;
                        }
                        case 2 : { //("R", "fcvt.s.l"), converts int64_t(rs1) to float(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)i64_to_f32( (int64_t)(fregs_x[rs1]) ).v;
                            break;
                        }
                        case 3 : { //("R", "fcvt.s.lu"), converts uint64_t(rs1) to float(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)ui64_to_f32( (uint64_t)(fregs_x[rs1]) ).v;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=104 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 105 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fcvt.d.w"), converts int32_t(rs1) to double(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)i32_to_f64( (int32_t)(fregs_x[rs1]) ).v;
                            break;
                        }
                        case 1 : { //("R", "fcvt.d.wu"), converts uint32_t(rs1) to double(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)ui32_to_f64( (uint32_t)(fregs_x[rs1]) ).v;
                            break;
                        }
                        case 2 : { //("R", "fcvt.d.l"), converts int64_t(rs1) to double(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)i64_to_f64( (int64_t)(fregs_x[rs1]) ).v;
                            break;
                        }
                        case 3 : { //("R", "fcvt.d.lu"), converts uint64_t(rs1) to double(rd)
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)ui64_to_f64( (uint64_t)(fregs_x[rs1]) ).v;
                            break;
                        }
                        default: // => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=105 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 112 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : { //("R", "fmv.x.w"), copies fregs(rs1) to regs(rd)
                                    uint64_t rd = (inst >> 7) & 0x1F;
                                    uint64_t rs1 = (inst >> 15) & 0x1F;
                                    fregs_x[rd] = fregs[rs1];
                                    break;
                                }
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                    FLOAT_ASSERT(false);
                                    break;
                            }
                            break;
                        }
                        /*
                        Format of result of FCLASS instruction.
                            rd bit  Meaning
                            0       rs1 is -infinite
                            1       rs1 is a negative normal number
                            2       rs1 is a negative subnormal number
                            3       rs1 is -0
                            4       rs1 is +0
                            5       rs1 is a positive subnormal number
                            6       rs1 is a positive normal number
                            7       rs1 is +infinite
                            8       rs1 is a signaling NaN
                            9       rs1 is a quiet NaN
                        */
                        case 1 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : { //("R", "fclass.s"),
                                    uint64_t rd = (inst >> 7) & 0x1F;
                                    if (rd != 0) {
                                        uint64_t rs1 = (inst >> 15) & 0x1F;
                                        if (F32_IS_MINUS_INFINITE(fregs[rs1]))
                                            fregs_x[rd] = (1 << 0); // negative infinite
                                        else if (F32_IS_PLUS_INFINITE(fregs[rs1]))
                                            fregs_x[rd] = (1 << 7); // positive infinite
                                        else if (F32_IS_MINUS_ZERO(fregs[rs1]))
                                            fregs_x[rd] = (1 << 3); // negative zero
                                        else if (F32_IS_PLUS_ZERO(fregs[rs1]))
                                            fregs_x[rd] = (1 << 4); // positive zero
                                        else if (F32_IS_QUIET_NAN(fregs[rs1]))
                                            fregs_x[rd] = (1 << 9); // quiet NaN
                                        else if (F32_IS_SIGNALING_NAN(fregs[rs1]))
                                            fregs_x[rd] = (1 << 8); // signaling NaN
                                        else if (F32_IS_SUBNORMAL(fregs[rs1]))
                                        {
                                            if (fregs[rs1] & F32_SIGN_BIT_MASK)
                                                fregs_x[rd] = (1 << 2); // negative subnormal
                                            else
                                                fregs_x[rd] = (1 << 5); // positive subnormal
                                        }
                                        else
                                        {
                                            FLOAT_ASSERT(F32_IS_NORMAL(fregs[rs1]));
                                            if (fregs[rs1] & F32_SIGN_BIT_MASK)
                                                fregs_x[rd] = (1 << 1); // negative normal
                                            else
                                                fregs_x[rd] = (1 << 6); // positive normal
                                        }
                                    }
                                    break;
                                }
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                    FLOAT_ASSERT(false);
                                    break;
                            }
                            break;
                        }
                        default: //_ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=112 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 113 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : { //("R", "fmv.x.d"), copies fregs(rs1) to regs(rd)
                                    uint64_t rd = (inst >> 7) & 0x1F;
                                    uint64_t rs1 = (inst >> 15) & 0x1F;
                                    fregs_x[rd] = fregs[rs1];
                                    break;
                                }
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                    FLOAT_ASSERT(false);
                                    break;
                            }
                            break;
                        }
                        /*
                        Format of result of FCLASS instruction.
                            rd bit  Meaning
                            0       rs1 is -infinite
                            1       rs1 is a negative normal number
                            2       rs1 is a negative subnormal number
                            3       rs1 is -0
                            4       rs1 is +0
                            5       rs1 is a positive subnormal number
                            6       rs1 is a positive normal number
                            7       rs1 is +infinite
                            8       rs1 is a signaling NaN
                            9       rs1 is a quiet NaN
                        */
                        case 1 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : { //("R", "fclass.d"),
                                    uint64_t rd = (inst >> 7) & 0x1F;
                                    if (rd != 0) {
                                        uint64_t rs1 = (inst >> 15) & 0x1F;
                                        if (fregs[rs1] == F64_MINUS_INFINITE)
                                            fregs_x[rd] = (1 << 0); // negative infinite
                                        else if (fregs[rs1] == F64_PLUS_INFINITE)
                                            fregs_x[rd] = (1 << 7); // positive infinite
                                        else if (fregs[rs1] == F64_MINUS_ZERO)
                                            fregs_x[rd] = (1 << 3); // negative zero
                                        else if (fregs[rs1] == F64_PLUS_ZERO)
                                            fregs_x[rd] = (1 << 4); // positive zero
                                        else if (F64_IS_QUIET_NAN(fregs[rs1]))
                                            fregs_x[rd] = (1 << 9); // quiet NaN
                                        else if (F64_IS_SIGNALING_NAN(fregs[rs1]))
                                            fregs_x[rd] = (1 << 8); // signaling NaN
                                        else if (F64_IS_SUBNORMAL(fregs[rs1]))
                                        {
                                            if (fregs[rs1] & F64_SIGN_BIT_MASK)
                                                fregs_x[rd] = (1 << 2); // negative subnormal
                                            else
                                                fregs_x[rd] = (1 << 5); // positive subnormal
                                        }
                                        else
                                        {
                                            FLOAT_ASSERT(F64_IS_NORMAL(fregs[rs1]));
                                            if (fregs[rs1] & F64_SIGN_BIT_MASK)
                                                fregs_x[rd] = (1 << 1); // negative normal
                                            else
                                                fregs_x[rd] = (1 << 6); // positive normal
                                        }
                                    }
                                    break;
                                }
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=113 funct3=0 inst=0x{inst:x}"),
                                    FLOAT_ASSERT(false);
                                    break;
                            }
                            break;
                        }
                        default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=112 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 120 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : { //("R", "fmv.w.x"), copies regs(rs1) to fregs(rd)
                                    uint64_t rd = (inst >> 7) & 0x1F;
                                    uint64_t rs1 = (inst >> 15) & 0x1F;
                                    fregs[rd] = fregs_x[rs1];
                                    break;
                                }
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=120 funct3=0 inst=0x{inst:x}"),
                                    FLOAT_ASSERT(false);
                                    break;
                            }
                            break;
                        }
                        default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=120 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                case 121 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : { //("R", "fmv.d.x"), copies regs(rs1) to fregs(rd)
                                    uint64_t rd = (inst >> 7) & 0x1F;
                                    uint64_t rs1 = (inst >> 15) & 0x1F;
                                    fregs[rd] = fregs_x[rs1];
                                    break;
                                }
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=121 funct3=0 inst=0x{inst:x}"),
                                    FLOAT_ASSERT(false);
                                    break;
                            }
                            break;
                        }
                        default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=121 inst=0x{inst:x}"),
                            FLOAT_ASSERT(false);
                            break;
                    }
                    break;
                }
                default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 83 inst=0x{inst:x}"),
                    FLOAT_ASSERT(false);
                    break;
            }
        }
    }
    /*
        softfloat_exceptionFlags:
        enum {
            softfloat_flag_inexact   =  1,
            softfloat_flag_underflow =  2,
            softfloat_flag_overflow  =  4,
            softfloat_flag_infinite  =  8,
            softfloat_flag_invalid   = 16
        };
    */

    // Update flags: copy flags from the library state register to fcsr
    fcsr = (fcsr & ~0x1F) | (softfloat_exceptionFlags & 0x1F);
    //fcsr = (softfloat_exceptionFlags & 0x1F) | ((softfloat_roundingMode & 0x7) << 5);
}

void set_rounding_mode (uint64_t rm)
{
    /*
    RISC-V spec:

    Rounding Mode Mnemonic Meaning
    ------------- -------- ---------------------------------------------------------
    000           RNE      Round to Nearest, ties to Even
    001           RTZ      Round towards Zero
    010           RDN      Round Down (towards -infinite)
    011           RUP      Round Up (towards +infinite)
    100           RMM      Round to Nearest, ties to Max Magnitude
    101                    Reserved for future use.
    110                    Reserved for future use.
    111           DYN      In instruction’s rm field, selects dynamic rounding mode;
                           In Rounding Mode register, reserved.
    
    SoftFloat library rounding mode enum:

    enum {
        softfloat_round_near_even   = 0,
        softfloat_round_minMag      = 1,
        softfloat_round_min         = 2,
        softfloat_round_max         = 3,
        softfloat_round_near_maxMag = 4,
        softfloat_round_odd         = 6
    };

    The mapping is direct but we want to ignore invalid values (5, 6, 7).
    */

    switch (rm & 0x7)
    {
        case 0: // RNE
            softfloat_roundingMode = softfloat_round_near_even;
            break;
        case 1: // RTZ
            softfloat_roundingMode = softfloat_round_minMag;
            break;
        case 2: // RDN
            softfloat_roundingMode = softfloat_round_min;
            break;
        case 3: // RUP
            softfloat_roundingMode = softfloat_round_max;
            break;
        case 4: // RMM
            softfloat_roundingMode = softfloat_round_near_maxMag;
            break;
        case 7: // DYN - should not be used in fcsr
        default:
            // Invalid rounding mode, do nothing
            break;
    }
}

void update_rounding_mode (uint64_t * rm)
{
    switch (*rm & 0x7)
    {
        case 0: // RNE
            break;
        case 1: // RTZ
            break;
        case 2: // RDN
            break;
        case 3: // RUP
            break;
        case 4: // RMM
            break;
        case 7: // DYN - get value from fcsr
            *rm = softfloat_roundingMode & 0x7;
            break;
        default:
            // Invalid rounding mode, do nothing
            break;
    }
}

void change_rounding_mode_sign (void)
{
    // Change the sign of the rounding mode in softfloat_roundingMode
    // This is a custom function not defined in RISC-V or SoftFloat specs.
    // It flips between RDN (2) and RUP (3), and leaves other modes unchanged.
    // This is done before calling SoftFloat functions which result will be negated.
    if (softfloat_roundingMode == softfloat_round_max)
        softfloat_roundingMode = softfloat_round_min;
    else if (softfloat_roundingMode == softfloat_round_min)
        softfloat_roundingMode = softfloat_round_max;
}

//uint64_t zisk_float_calls = 0;

#ifdef __cplusplus
} // extern "C"
#endif