//#include <stdint.h>
#include "softfloat.h"
#include "float.h"

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_ASSERT(condition) \
    do { \
        if (!(condition)) { \
            *(uint64_t *)0x0 = 0; \
        } \
    } while (0)

void set_rounding_mode (uint64_t rm);
void update_rounding_mode (uint64_t * rm);

void _zisk_float (void)
{
    // Before calling any softfloat function, set the rounding mode from the fcsr register
    // into the softfloat_roundingMode variable.
    softfloat_roundingMode = (fcsr >> 5) & 0x7;

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
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v;
                    break;
                }
                case 1: { //=> ("R4", "fmadd.d"), rd = (rs1 x rs2) + rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
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
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){NEG32(fregs[rs3])} ).v;
                    break;
                }
                case 1: { //=> ("R4", "fmsub.d"), rd = (rs1 x rs2) - rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
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
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f32_mulAdd( (float32_t){NEG32(fregs[rs1])}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v;
                    break;
                }
                case 1: { //=> ("R4", "fnmsub.d"), rd = -(rs1 x rs2) + rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
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
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)NEG32(f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v);
                    break;
                }
                case 1: { //=> ("R4", "fnmadd.d"), rd = -(rs1 x rs2) - rs3
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
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
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f32_add( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 1 : { //("R", "fadd.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f64_add( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 4 : { //("R", "fsub.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f32_sub( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 5 : { //("R", "fsub.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f64_sub( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 8 : { //("R", "fmul.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f32_mul( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 9 : { //("R", "fmul.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f64_mul( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 12 : { //("R", "fdiv.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
                    fregs[rd] = (uint64_t)f32_div( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 13 : { //("R", "fdiv.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7;
                    set_rounding_mode(rm);
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
                            fregs[rd] = f32_lt( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? fregs[rs1] : fregs[rs2];
                            break;
                        }
                        case 1 : { //("R", "fmax.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
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
                            fregs[rd] = f64_lt( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? fregs[rs1] : fregs[rs2];
                            break;
                        }
                        case 1 : { //("R", "fmax.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
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
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
                            fregs[rd] = (uint64_t)f64_to_f32( (float64_t){fregs[rs1]} ).v;
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
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7;
                            set_rounding_mode(rm);
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
                                    uint64_t rs1 = (inst >> 15) & 0x1F;
                                    fregs_x[rd] = 0;
                                    if (fregs[rs1] == F32_MINUS_INFINITE)
                                        fregs_x[rd] |= (1 << 0);
                                    else if (fregs[rs1] == F32_PLUS_INFINITE)
                                        fregs_x[rd] |= (1 << 7);
                                    else if (fregs[rs1] == F32_MINUS_ZERO)
                                        fregs_x[rd] |= (1 << 3);
                                    else if (fregs[rs1] == F32_PLUS_ZERO)
                                        fregs_x[rd] |= (1 << 4);
                                    else if ( (fregs[rs1] & F32_EXPONENT_MASK) != 0 && (fregs[rs1] & F32_EXPONENT_MASK) != F32_EXPONENT_MASK ) // not zero or inf or NaN
                                    {
                                        if (fregs[rs1] & F32_SIGN_BIT_MASK)
                                            fregs_x[rd] |= (1 << 1); // negative normal
                                        else
                                            fregs_x[rd] |= (1 << 6); // positive normal
                                    }
                                    else if ( (fregs[rs1] & F32_EXPONENT_MASK) == 0 && (fregs[rs1] & F32_MANTISSA_MASK) != 0 ) // subnormal
                                    {
                                        if (fregs[rs1] & F32_SIGN_BIT_MASK)
                                            fregs_x[rd] |= (1 << 2); // negative subnormal
                                        else
                                            fregs_x[rd] |= (1 << 5); // positive subnormal
                                    }
                                    else if ( ((fregs[rs1] & F32_EXPONENT_MASK) == F32_EXPONENT_MASK) && ((fregs[rs1] & F32_QUIET_NAN_MASK) == 0) )
                                        fregs_x[rd] |= (1 << 8); // signaling NaN
                                    else if ( ((fregs[rs1] & F32_EXPONENT_MASK) == F32_EXPONENT_MASK) && ((fregs[rs1] & F32_QUIET_NAN_MASK) != 0) )
                                        fregs_x[rd] |= (1 << 9); // quiet NaN
                                    else
                                        ; // should not happen
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
                                    uint64_t rs1 = (inst >> 15) & 0x1F;
                                    fregs_x[rd] = 0;
                                    if (fregs[rs1] == F64_MINUS_INFINITE)
                                        fregs_x[rd] |= (1 << 0);
                                    else if (fregs[rs1] == F64_PLUS_INFINITE)
                                        fregs_x[rd] |= (1 << 7);
                                    else if (fregs[rs1] == F64_MINUS_ZERO)
                                        fregs_x[rd] |= (1 << 3);
                                    else if (fregs[rs1] == F64_PLUS_ZERO)
                                        fregs_x[rd] |= (1 << 4);
                                    else if ( (fregs[rs1] & F64_EXPONENT_MASK) != 0 && (fregs[rs1] & F64_EXPONENT_MASK) != F64_EXPONENT_MASK ) // not zero or inf or NaN
                                    {
                                        if (fregs[rs1] & F64_SIGN_BIT_MASK)
                                            fregs_x[rd] |= (1 << 1); // negative normal
                                        else
                                            fregs_x[rd] |= (1 << 6); // positive normal
                                    }
                                    else if ( (fregs[rs1] & F64_EXPONENT_MASK) == 0 && (fregs[rs1] & F64_MANTISSA_MASK) != 0 ) // subnormal
                                    {
                                        if (fregs[rs1] & F64_SIGN_BIT_MASK)
                                            fregs_x[rd] |= (1 << 2); // negative subnormal
                                        else
                                            fregs_x[rd] |= (1 << 5); // positive subnormal
                                    }
                                    else if ( ((fregs[rs1] & F64_EXPONENT_MASK) == F64_EXPONENT_MASK) && ((fregs[rs1] & F64_QUIET_NAN_MASK) == 0) )
                                        fregs_x[rd] |= (1 << 8); // signaling NaN
                                    else if ( ((fregs[rs1] & F64_EXPONENT_MASK) == F64_EXPONENT_MASK) && ((fregs[rs1] & F64_QUIET_NAN_MASK) != 0) )
                                        fregs_x[rd] |= (1 << 9); // quiet NaN
                                    else
                                        ; // should not happen
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

#ifdef __cplusplus
} // extern "C"
#endif