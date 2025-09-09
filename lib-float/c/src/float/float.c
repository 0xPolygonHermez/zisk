//#include <stdint.h>
#include "softfloat.h"

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
const uint64_t FREG_CSR = FREG_FIRST + 32 * 8; // Floating-point control and status register (fcsr)
const uint64_t FREG_INST = FREG_FIRST + 33 * 8; // Floating-point instruction register (finst)
static uint64_t myvalue = 0x3ff3333333333333; // 1.7
uint64_t * regs = (uint64_t *)REG_FIRST;
uint64_t * fregs = (uint64_t *)FREG_FIRST;

// Negate a float by flipping its sign bit(s)
const uint64_t SIGN_BIT_MASK_64 = 0x8000000000000000;
const uint64_t SIGN_BIT_MASK_32 = 0xFFFFFFFF80000000;
#define NEG64(x) ((x) ^ SIGN_BIT_MASK_64)
#define NEG32(x) ((x) ^ SIGN_BIT_MASK_32)

// 1.0 and 0.0 in IEEE 754 format
const uint64_t F64_ONE = 0x3FF0000000000000;
const uint64_t F32_ONE = 0x3F800000;
const uint64_t F64_ZERO = 0x0000000000000000;
const uint32_t F32_ZERO = 0x00000000;

void zisk_float (void)
{
    // uint64_t inst = *(uint64_t *)FREG_INST;
    // uint64_t * freg = (uint64_t *)FREG_F0;
    // for (int i = 0; i < 32; i++)
    //     freg[i] = i;

    //fregs[3] = (uint64_t)f64_add( (float64_t){fregs[1]}, (float64_t){fregs[2]} ).v;
    //fregs[3] = myvalue;
    ((uint64_t *)FREG_F0)[3] = myvalue;
    myvalue = myvalue + 1;


    uint64_t inst = *(uint64_t *)FREG_INST;
    switch (inst & 0x7F)
    {
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
                case 0: { //("R4", "fmadd.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v;
                    break;
                }
                case 1: { //=> ("R4", "fmadd.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f64_mulAdd( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]}, (float64_t){fregs[rs3]} ).v;
                    break;
                }
                default: //_ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 67 inst=0x{inst:x}"),
                    break;
            }
            break;
        }

        case 71 : { // Opcode 71
            switch ((inst >> 25) & 0x3) {
                case 0: { //("R4", "fmsub.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){NEG32(fregs[rs2])}, (float32_t){fregs[rs3]} ).v;
                    break;
                }
                case 1: { //=> ("R4", "fmsub.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f64_mulAdd( (float64_t){fregs[rs1]}, (float64_t){NEG64(fregs[rs2])}, (float64_t){fregs[rs3]} ).v;
                    break;
                }
                default: //_ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 71 inst=0x{inst:x}"),
                    break;
            }
            break;
        }

        case 75 : { // Opcode 75
            switch ((inst >> 25) & 0x3) {
                case 0: { //("R4", "fnmsub.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)NEG32(f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){NEG32(fregs[rs2])}, (float32_t){fregs[rs3]} ).v);
                    break;
                }
                case 1: { //=> ("R4", "fnmsub.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)NEG64(f64_mulAdd( (float64_t){fregs[rs1]}, (float64_t){NEG64(fregs[rs2])}, (float64_t){fregs[rs3]} ).v);
                    break;
                }
                default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 75 inst=0x{inst:x}"),
                    break;
            }
        }

        case 79 : { // Opcode 79
            switch ((inst >> 25) & 0x3) {
                case 0: { //("R4", "fnmadd.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)NEG32(f32_mulAdd( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]}, (float32_t){fregs[rs3]} ).v);
                    break;
                }
                case 1: { //=> ("R4", "fnmadd.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rs3 = (inst >> 27) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)NEG64(f64_mulAdd( (float64_t){fregs[rs1]}, (float64_t){NEG64(fregs[rs2])}, (float64_t){fregs[rs3]} ).v);
                    break;
                }
                default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 79 inst=0x{inst:x}"),
                    break;
            }
        }

        case 83 : { // Opcode 83
            switch ((inst >> 25) & 0x7F) {
                case 0 : { //("R", "fadd.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f32_add( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 1 : { //("R", "fadd.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f64_add( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 4 : { //("R", "fsub.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f32_sub( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 5 : { //("R", "fsub.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f64_sub( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 8 : { //("R", "fmul.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[3] = (uint64_t)f32_mul( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 9 : { //("R", "fmul.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[3] = (uint64_t)f64_mul( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 12 : { //("R", "fdiv.s"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[3] = (uint64_t)f32_div( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 13 : { //("R", "fdiv.d"),
                    uint64_t rd = (inst >> 7) & 0x1F;
                    uint64_t rs1 = (inst >> 15) & 0x1F;
                    uint64_t rs2 = (inst >> 20) & 0x1F;
                    uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                    fregs[3] = (uint64_t)f64_div( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 16 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : //("R", "fsgnj.s"),
                        case 1 : //("R", "fsgnjn.s"),
                        case 2 : //("R", "fsgnjx.s"),
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=16 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 17 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : //("R", "fsgnj.d"),
                        case 1 : //("R", "fsgnjn.d"),
                        case 2 : //("R", "fsgnjx.d"),
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=17 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 20 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : { //("R", "fmin.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f32_lt( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? fregs[rs1] : fregs[rs2];
                            break;
                        }
                        case 1 : { //("R", "fmax.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f32_lt( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? fregs[rs2] : fregs[rs1];
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=20 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 21 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : { //("R", "fmin.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f64_lt( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? fregs[rs1] : fregs[rs2];
                            break;
                        }
                        case 1 : { //("R", "fmax.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f64_lt( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? fregs[rs2] : fregs[rs1];
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=21 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 32 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 1 : //("R", "fcvt.s.d"),
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=32 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 33 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : //("R", "fcvt.d.s"),
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=33 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 44 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fsqrt.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                            fregs[3] = (uint64_t)f32_sqrt( (float32_t){fregs[rs1]} ).v;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=44 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 45 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : { //("R", "fsqrt.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rm = (inst >> 12) & 0x7; // TODO: use rm
                            fregs[3] = (uint64_t)f64_sqrt( (float64_t){fregs[rs1]} ).v;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=45 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 80 : {
                    switch ((inst >> 12) & 0x7) {
                        case 2 : { //("R", "feq.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f32_eq( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? F32_ONE : F32_ZERO;
                            break;
                        }
                        case 1 : { //("R", "flt.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f32_lt( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? F32_ONE : F32_ZERO;
                            break;
                        }
                        case 0 : { //("R", "fle.s"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f32_le( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ) ? F32_ONE : F32_ZERO;
                            break;
                        }
                        default: // => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=80 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 81 : {
                    switch ((inst >> 12) & 0x7) {
                        case 2 : { //("R", "feq.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f64_eq( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? F64_ONE : F64_ZERO;
                            break;
                        }
                        case 1 : { //("R", "flt.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f64_lt( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? F64_ONE : F64_ZERO;
                            break;
                        }
                        case 0 : { //("R", "fle.d"),
                            uint64_t rd = (inst >> 7) & 0x1F;
                            uint64_t rs1 = (inst >> 15) & 0x1F;
                            uint64_t rs2 = (inst >> 20) & 0x1F;
                            fregs[3] = f64_le( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ) ? F64_ONE : F64_ZERO;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=81 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 96: {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : //("R", "fcvt.w.s"),
                        case 1 : //("R", "fcvt.wu.s"),
                        case 2 : //("R", "fcvt.l.s"),
                        case 3 : //("R", "fcvt.lu.s"),
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=96 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 97 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : //("R", "fcvt.w.d"), converts double(rs1) to int32_t(rd)
                        case 1 : //("R", "fcvt.wu.d"), converts double(rs1) to uint32_t(rd)
                        case 2 : //("R", "fcvt.l.d"), converts double(rs1) to int64_t(rd)
                        case 3 : //("R", "fcvt.lu.d"), converts double(rs1) to uint64_t(rd)
                        default: // => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=97 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 104 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : //("R", "fcvt.s.w"),
                        case 1 : //("R", "fcvt.s.wu"),
                        case 2 : //("R", "fcvt.s.l"),
                        case 3 : //("R", "fcvt.s.lu"),
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=104 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 105 : {
                    switch ((inst >> 20) & 0x1F) {
                        case 0 : //("R", "fcvt.d.w"), converts int32_t(rs1) to double(rd)
                        case 1 : //("R", "fcvt.d.wu"), converts uint32_t(rs1) to double(rd)
                        case 2 : //("R", "fcvt.d.l"), converts int64_t(rs1) to double(rd)
                        case 3 : //("R", "fcvt.d.lu"), converts uint64_t(rs1) to double(rd)
                        default: // => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=105 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 112 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : //("R", "fmv.x.w"),
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                    break;
                            }
                        }
                        case 1 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : //("R", "fclass.s"),
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                    break;
                            }
                        }
                        default: //_ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=112 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 113 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : //("R", "fmv.x.d"),
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                    break;
                            }
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
                                case 0 : //("R", "fclass.d"),
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=113 funct3=0 inst=0x{inst:x}"),
                                    break;
                            }
                        }
                        default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=112 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 120 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : //("I", "fmv.w.x"),
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=120 funct3=0 inst=0x{inst:x}"),
                                    break;
                            }
                        }
                        default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=120 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 121 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : {
                            switch ((inst >> 20) & 0x1F) {
                                case 0 : //("I", "fmv.d.x"),
                                default: // panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=121 funct3=0 inst=0x{inst:x}"),
                                    break;
                            }
                        }
                        default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=121 inst=0x{inst:x}"),
                            break;
                    }
                }
                default: // panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 83 inst=0x{inst:x}"),
                    break;
            }
        }
    }
}

#ifdef __cplusplus
} // extern "C"
#endif