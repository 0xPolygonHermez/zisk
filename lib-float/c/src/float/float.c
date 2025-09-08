//#include <stdint.h>
#include "softfloat.h"

#ifdef __cplusplus
extern "C" {
#endif
#define uint64_t unsigned long long
// System address where the floating-point registers are mapped
const uint64_t SYS_ADDR = 0xa0000000;
const uint64_t FREG_OFFSET = 40;
const uint64_t FREG_FIRST = SYS_ADDR + FREG_OFFSET * 8;
const uint64_t FREG_F0 = FREG_FIRST;
const uint64_t FREG_CSR = FREG_FIRST + 32 * 8; // Floating-point control and status register (fcsr)
const uint64_t FREG_INST = FREG_FIRST + 33 * 8; // Floating-point instruction register (finst)
static uint64_t myvalue = 0x3ff3333333333333; // 1.7
uint64_t * fregs = (uint64_t *)FREG_F0;

//void zisk_float (uint64_t * fregs)
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
                case 0: //("R4", "fmadd.s"),
                case 1: //=> ("R4", "fmadd.d"),
                default: //_ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 67 inst=0x{inst:x}"),
                    break;
            }
            break;
        }

        case 71 : { // Opcode 71
            switch ((inst >> 25) & 0x3) {
                case 0: //("R4", "fmsub.s"),
                case 1: //=> ("R4", "fmsub.d"),
                default: //_ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 71 inst=0x{inst:x}"),
                    break;
            }
            break;
        }

        case 75 : { // Opcode 75
            switch ((inst >> 25) & 0x3) {
                case 0: //("R4", "fnmsub.s"),
                case 1: //=> ("R4", "fnmsub.d"),
                default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 75 inst=0x{inst:x}"),
                    break;
            }
        }

        case 79 : { // Opcode 79
            switch ((inst >> 25) & 0x3) {
                case 0: //("R4", "fnmadd.s"),
                case 1: //=> ("R4", "fnmadd.d"),
                default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 79 inst=0x{inst:x}"),
                    break;
            }
        }

        case 83 : { // Opcode 83
            switch ((inst >> 25) & 0x7F) {
                case 0 : { //("R", "fadd.s"),
                    uint64_t rd = (inst >> 7) && 0x1F;
                    uint64_t rs1 = (inst >> 15) && 0x1F;
                    uint64_t rs2 = (inst >> 20) && 0x1F;
                    uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f32_add( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 1 : { //("R", "fadd.d"),
                    uint64_t rd = (inst >> 7) && 0x1F;
                    uint64_t rs1 = (inst >> 15) && 0x1F;
                    uint64_t rs2 = (inst >> 20) && 0x1F;
                    uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f64_add( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 4 : { //("R", "fsub.s"),
                    uint64_t rd = (inst >> 7) && 0x1F;
                    uint64_t rs1 = (inst >> 15) && 0x1F;
                    uint64_t rs2 = (inst >> 20) && 0x1F;
                    uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f32_sub( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 5 : { //("R", "fsub.d"),
                    uint64_t rd = (inst >> 7) && 0x1F;
                    uint64_t rs1 = (inst >> 15) && 0x1F;
                    uint64_t rs2 = (inst >> 20) && 0x1F;
                    uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
                    fregs[rd] = (uint64_t)f64_sub( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 8 : { //("R", "fmul.s"),
                    uint64_t rd = (inst >> 7) && 0x1F;
                    uint64_t rs1 = (inst >> 15) && 0x1F;
                    uint64_t rs2 = (inst >> 20) && 0x1F;
                    uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
                    fregs[3] = (uint64_t)f32_mul( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 9 : { //("R", "fmul.d"),
                    uint64_t rd = (inst >> 7) && 0x1F;
                    uint64_t rs1 = (inst >> 15) && 0x1F;
                    uint64_t rs2 = (inst >> 20) && 0x1F;
                    uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
                    fregs[3] = (uint64_t)f64_mul( (float64_t){fregs[rs1]}, (float64_t){fregs[rs2]} ).v;
                    break;
                }
                case 12 : { //("R", "fdiv.s"),
                    uint64_t rd = (inst >> 7) && 0x1F;
                    uint64_t rs1 = (inst >> 15) && 0x1F;
                    uint64_t rs2 = (inst >> 20) && 0x1F;
                    uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
                    fregs[3] = (uint64_t)f32_div( (float32_t){fregs[rs1]}, (float32_t){fregs[rs2]} ).v;
                    break;
                }
                case 13 : { //("R", "fdiv.d"),
                    uint64_t rd = (inst >> 7) && 0x1F;
                    uint64_t rs1 = (inst >> 15) && 0x1F;
                    uint64_t rs2 = (inst >> 20) && 0x1F;
                    uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
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
                        case 0 : //("R", "fmin.s"),
                        case 1 : //("R", "fmax.s"),
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=20 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 21 : {
                    switch ((inst >> 12) & 0x7) {
                        case 0 : //("R", "fmin.d"),
                        case 1 : //("R", "fmax.d"),
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
                            uint64_t rd = (inst >> 7) && 0x1F;
                            uint64_t rs1 = (inst >> 15) && 0x1F;
                            uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
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
                            uint64_t rd = (inst >> 7) && 0x1F;
                            uint64_t rs1 = (inst >> 15) && 0x1F;
                            uint64_t rm = (inst >> 12) && 0x7; // TODO: use rm
                            fregs[3] = (uint64_t)f64_sqrt( (float64_t){fregs[rs1]} ).v;
                            break;
                        }
                        default: //=> panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=45 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 80 : {
                    switch ((inst >> 12) & 0x7) {
                        case 2 : //("R", "feq.s"),
                        case 1 : //("R", "flt.s"),
                        case 0 : //("R", "fle.s"),
                        default: // => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=80 inst=0x{inst:x}"),
                            break;
                    }
                }
                case 81 : {
                    switch ((inst >> 12) & 0x7) {
                        case 2 : //("R", "feq.d"),
                        case 1 : //("R", "flt.d"),
                        case 0 : //("R", "fle.d"),
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
                        case 0 : //("R", "fcvt.w.d"),
                        case 1 : //("R", "fcvt.wu.d"),
                        case 2 : //("R", "fcvt.l.d"),
                        case 3 : //("R", "fcvt.lu.d"),
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
                        case 0 : //("R", "fcvt.d.w"),
                        case 1 : //("R", "fcvt.d.wu"),
                        case 2 : //("R", "fcvt.d.l"),
                        case 3 : //("R", "fcvt.d.lu"),
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