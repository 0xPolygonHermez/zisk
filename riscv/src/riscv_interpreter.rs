//! Parses a 32-bits RISC-V instruction

use crate::{RiscvInstruction, Rvd};

/// Convert 32-bits data chunk that contains a signed integer of a specified size in bits to a
/// signed integer of 32 bits
fn signext(v: u32, size: u32) -> i32 {
    let sign_bit: u32 = 1u32 << (size - 1);
    let max_value: u32 = 1u32 << size;
    if (sign_bit & v) != 0 {
        v as i32 - max_value as i32
    } else {
        v as i32
    }
}

/// Interprets a buffer of 32-bits RICSV instructions into a vector of decoded RISCV instructions
/// split by field
pub fn riscv_interpreter(code: &[u16]) -> Vec<RiscvInstruction> {
    let mut insts = Vec::<RiscvInstruction>::new();

    // code_len is the length of the input code buffer,
    // which can contain both 16-bit and 32-bit instructions
    let code_len = code.len();

    // code_index is the index in the code buffer, from 0 to code_len - 1
    let mut code_index: usize = 0;

    // For every 16-bit instruction in the input code buffer
    while code_index < code_len {
        //println!("riscv_interpreter() code_index={}", code_index);

        // Get the RISCV instruction
        let inst = code[code_index];
        code_index += 1;

        // Ignore instructions that are zero
        // As per spec, they can only be 32 bits instructions, so we need to read the next 16 bits
        // and check that they are also zero
        if inst == 0 {
            let inst = code[code_index];
            code_index += 1;
            assert!(
                inst == 0,
                "riscv_interpreter() found inst!=0 after 0 at position code_index={} (index in u16 array)",
                code_index - 1
            );
            // println!("riscv_interpreter() found inst=0 at position s={} (index in u32 array)", s);
            insts.push(RiscvInstruction::nop(0));
            continue;
        }

        /***********/
        /* 32 bits */
        /***********/
        // If this is a 32 bits instruction, then we need to read the next 16 bits
        if (inst & 0x3) == 0x3 {
            // Build a 32-bit instruction from two consecutive 16-bit instructions
            if code_index > code_len - 1 {
                panic!("riscv_interpreter() found incomplete 32-bits instruction at the end of the code buffer at index={code_index}");
            }
            let inst: u32 = (inst as u32) | ((code[code_index] as u32) << 16);
            code_index += 1;

            let (inst_type, inst_name) = Rvd::get_type_and_name_32_bits(inst);

            // Create a RISCV instruction instance with the known fields to be filled with data
            // from the instruction based on its format type
            let mut i = RiscvInstruction {
                rvinst: inst,
                t: inst_type.to_string(),
                inst: inst_name.to_string(),
                ..Default::default()
            };

            // Decode the rest of instruction fields based on the instruction type

            //  31 30 ... 21 20 19 ... 15 14 13 12 11 ... 07 06 05 04 03 02 01 00
            // |  imm[11:0]    |  rs1    | funct3 |   rd    |       opcode       | I-type
            if i.t == *"I" {
                i.funct3 = (inst & 0x7000) >> 12;
                //let funct7 = (inst & 0xFC000000) >> 26;
                i.funct7 = (inst & 0xFC000000) >> 26;
                i.rd = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.imm = signext((inst & 0xFFF00000) >> 20, 12);
                // let l: i32;
                // (i.inst, l) = getinst(&inf.op, i.funct3, funct7);
                // assert!(
                //     !i.inst.is_empty(),
                //     "i.inst.is_empty() for inst=0x{:x} at index={} opcode={} func3={:?} funct7={:?}",
                //     inst,
                //     code_index,
                //     opcode,
                //     i.funct3,
                //     funct7
                // );
                // if l == 2 {
                //     i.imm &= 0x3F;
                //     i.funct7 = funct7;
                // }
            }
            //  31 ... 27  26 25 24 ... 20 19 ... 15 14 13 12 11 ... 07 06 05 04 03 02 01 00
            // |  rs3    |funct2| rs2    |  rs1    | funct3 |   rd    |       opcode       | R-type
            else if i.t == *"R" {
                i.funct3 = (inst & 0x7000) >> 12;
                i.rd = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                i.rs3 = inst >> 27;
                i.funct2 = (inst >> 25) & 0x3;
            }
            //  31 30 ... 26 25 24 ... 20 19 ... 15 14 13 12 11 ... 07 06 05 04 03 02 01 00
            // |   funct7      |  rs2    |  rs1    | funct3 |   rd    |       opcode       | R-type
            else if i.t == *"R" {
                i.funct3 = (inst & 0x7000) >> 12;
                i.rd = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                i.funct7 = (inst & 0xFE000000) >> 25;
            }
            //  31 30 ... 26 25 24 ... 20 19 ... 15 14 13 12 11 10 09 08 07 06 05 04 03 02 01 00
            // |  imm[11:5]    |  rs2    |   rs1   | funct3 |   imm[4:0]   |       opcode       | S-type
            else if i.t == *"S" {
                i.funct3 = (inst & 0x7000) >> 12;
                let imm4_0 = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                let imm11_5 = (inst & 0xFE000000) >> 25;
                i.imm = signext((imm11_5 << 5) | imm4_0, 12);
            }
            //  31 30 29 28 27 26 25 24...20 19...15 14 13 12 11 10 09 08 07 06 05 04 03 02 01 00
            // |12|    imm[10:5]    |  rs2  | rs1   | funct3 |imm[4:1]   |11|       opcode       | B-type
            else if i.t == *"B" {
                i.funct3 = (inst & 0x7000) >> 12;
                let imm11 = (inst & 0x080) >> 7;
                let imm4_1 = (inst & 0xF00) >> 8;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                let imm10_5 = (inst & 0x7E000000) >> 25;
                let imm12 = (inst & 0x80000000) >> 31;
                i.imm = signext((imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1), 13);
            }
            //  31 30 ... 13 12 11 10 09 08 07 06 05 04 03 02 01 00
            // |  imm[31:12]   |      rd      |        opcode      | U-type
            else if i.t == *"U" {
                i.rd = (inst & 0xF80) >> 7;
                i.imm = (((inst & 0xFFFFF000) >> 12) << 12) as i32;
            }
            //  31 30 29...22 21 20 19 18 ... 13 12 11 10 09 08 07 06 05 04 03 02 01 00
            // |20|  imm[10:1]  |11|  imm[19:12]   |      rd      |       opcode       | J-type
            else if i.t == *"J" {
                i.rd = (inst & 0xF80) >> 7;
                let imm20 = (inst & 0x80000000) >> 31;
                let imm10_1 = (inst & 0x7FE00000) >> 21;
                let imm11j = (inst & 0x100000) >> 20;
                let imm19_12 = (inst & 0xFF000) >> 12;
                i.imm =
                    signext((imm20 << 20) | (imm19_12 << 12) | (imm11j << 11) | (imm10_1 << 1), 21);
            } else if i.t == *"A" {
                i.funct3 = (inst & 0x7000) >> 12;
                i.rd = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                i.funct5 = (inst & 0xF8000000) >> 27;
                i.aq = (inst & 0x4000000) >> 26;
                i.rl = (inst & 0x2000000) >> 24;
            } else if i.t == *"C" {
                i.funct3 = (inst & 0x7000) >> 12;
                if i.funct3 == 0 {
                    if inst == 0x00000073 {
                        i.inst = "ecall".to_string();
                    } else if inst == 0x00100073 {
                        i.inst = "ebreak".to_string();
                    } else {
                        i.inst = "ecall".to_string();
                        // TODO check what means this extra bits in ECALL
                        // throw new Error(`Invalid opcode: ${opcode} at index=${code_index}`);
                    }
                } else {
                    i.rd = (inst & 0xF80) >> 7;
                    if (i.funct3 & 0x4) != 0 {
                        i.imme = (inst & 0xF8000) >> 15;
                    } else {
                        i.rs1 = (inst & 0xF8000) >> 15;
                    }
                    i.csr = (inst & 0xFFF00000) >> 20;
                }
            } else if i.t == *"F" {
                i.funct3 = (inst & 0x7000) >> 12;
                if i.funct3 == 0 {
                    if (inst & 0xF00F8F80) != 0 {
                        panic!("Invalid F funct3=0 inst=0x{inst:x} at index={code_index}");
                    }
                    i.pred = (inst & 0x0F000000) >> 24;
                    i.succ = (inst & 0x00F00000) >> 20;
                    i.inst = "fence".to_string();
                } else if i.funct3 == 1 {
                    if (inst & 0xFFFF8F80) != 0 {
                        panic!("Invalid F funct3=1 inst=0x{inst:x} at index={code_index}");
                    }
                    i.inst = "fence.i".to_string();
                } else {
                    panic!("Invalid F funct3={:?} inst=0x{inst:x} at index={code_index}", i.funct3);
                }
            } else {
                panic!("Invalid i.t={} at index={}", i.t, code_index);
            }
            insts.push(i);
        }
        /***********/
        /* 16 bits */
        /***********/
        else {
            // This is a 16-bit instruction, so we need to decode it accordingly
            let (inst_type, inst_name) = Rvd::get_type_and_name_16_bits(inst);

            // Create a RISCV instruction instance to be filled with data from the instruction and from
            // the RVD info data
            // Copy the original RISCV 32-bit instruction
            // Copy the instruction type
            let mut i = RiscvInstruction {
                rvinst: inst as u32,
                t: inst_type.to_string(),
                inst: inst_name.to_string(),
                ..Default::default()
            };

            // Decode the rest of instruction fields based on the instruction type

            if i.t == "CR" {
                // Format Meaning              |15 14 13 12  |11 10 9 8 7 |6 5 4 3 2 |1 0|
                // CR     Register             |funct4       |rd/rs1      |rs2       |op |
                i.rs1 = ((inst >> 7) & 0x1F) as u32;
                i.rs2 = ((inst >> 2) & 0x1F) as u32;

                if inst_name == "c.jr" {
                    i.rd = 0;
                    if i.rs2 != 0 {
                        panic!("Invalid use of rs2!=0 in c.jr at index={code_index}");
                    }
                } else if inst_name == "c.jalr" {
                    i.rd = 1;
                } else if inst_name == "c.mv" {
                    i.rd = i.rs1;
                    i.rs1 = 0;
                    if i.rd == 0 {
                        // This is a hint and must not be executed
                        i.inst = "c.nop".to_string(); // Change to c.nop
                    }
                } else {
                    i.rd = i.rs1;
                }
            } else if i.t == "CI" {
                // Format Meaning              |15 14 13 |12  |11 10 9 8 7 |6 5 4 3 2 |1 0|
                // CI     Immediate            |funct3   |imm |rd/rs1      |imm       |op |
                i.rd = ((inst >> 7) & 0x1F) as u32;
                i.rs1 = i.rd;
                if inst_name == "c.addi16sp" {
                    let imm9 = ((inst >> 12) & 0x1) as u32;
                    let imm4 = ((inst >> 6) & 0x1) as u32;
                    let imm6 = ((inst >> 5) & 0x1) as u32;
                    let imm8_7 = ((inst >> 3) & 0x3) as u32;
                    let imm5 = ((inst >> 2) & 0x1) as u32;
                    let imm = (imm9 << 9) | (imm8_7 << 7) | (imm6 << 6) | (imm5 << 5) | (imm4 << 4);
                    i.imm = signext(imm, 10);
                } else if (inst_name == "c.addi") || (inst_name == "c.addiw") {
                    let imm5 = ((inst >> 12) & 0x1) as u32;
                    let imm4_0 = ((inst >> 2) & 0x1F) as u32;
                    let imm = (imm5 << 5) | imm4_0;
                    i.imm = signext(imm, 6);
                    if i.rd == 0 {
                        // This is a hint and must not be executed
                        i.inst = "c.nop".to_string(); // Change to c.nop
                    }
                } else if inst_name == "c.li" {
                    if i.rd == 0 {
                        // This is a hint and must not be executed
                        i.inst = "c.nop".to_string(); // Change to c.nop
                    } else {
                        let imm5 = ((inst >> 12) & 0x1) as u32;
                        let imm4_0 = ((inst >> 2) & 0x1F) as u32;
                        let imm = (imm5 << 5) | imm4_0;
                        i.imm = signext(imm, 6);
                        i.rs1 = 0;
                    }
                } else if inst_name == "c.lui" {
                    let imm17 = ((inst >> 12) & 0x1) as u32;
                    let imm16_12 = ((inst >> 2) & 0x1F) as u32;
                    i.imm = signext((imm17 << 17) | (imm16_12 << 12), 18);
                    if i.rd == 0 {
                        // This is a hint and must not be executed
                        i.inst = "c.nop".to_string(); // Change to c.nop
                    }
                    if i.rd == 2 {
                        panic!(
                            "Invalid use of rd=2 in c.lui at index={} inst=0x{:x}",
                            code_index, inst
                        );
                    }
                } else if inst_name == "c.ldsp" {
                    let imm5 = ((inst >> 12) & 0x1) as u32;
                    let imm4_3 = ((inst >> 5) & 0x3) as u32;
                    let imm8_6 = ((inst >> 2) & 0x7) as u32;
                    i.imm = ((imm8_6 << 6) | (imm5 << 5) | (imm4_3 << 3)) as i32;
                    if i.rd == 0 {
                        panic!("Invalid use of rd=0 in c.ldsp at index={code_index}");
                    }
                    i.rs1 = 2; // x2 is always the base pointer for LDSP instructions
                } else if inst_name == "c.lwsp" {
                    let imm5 = ((inst >> 12) & 0x1) as u32;
                    let imm4_2 = ((inst >> 4) & 0x7) as u32;
                    let imm7_6 = ((inst >> 2) & 0x3) as u32;
                    i.imm = ((imm7_6 << 6) | (imm5 << 5) | (imm4_2 << 2)) as i32;
                    if i.rd == 0 {
                        panic!("Invalid use of rd=0 in c.lwsp at index={code_index}");
                    }
                    i.rs1 = 2; // x2 is always the base pointer for LWSP instructions
                } else {
                    let imm5 = ((inst >> 12) & 0x1) as u32;
                    let imm4_0 = ((inst >> 2) & 0x1F) as u32;
                    i.imm = ((imm5 << 5) | imm4_0) as i32;
                }
            } else if i.t == "CSS" {
                // Format Meaning              |15 14 13 |12  11 10 9 8 7 |6 5 4 3 2 |1 0|
                // CSS    Stack-relative Store |funct3   |imm             |rs2       |op |

                // imm format depends on func3:
                // 101 imm[5:3|8:6] rs2 10 C.FSDSP (RV32/64)
                // 110 imm[5:2|7:6] rs2 10 C.SWSP
                // 111 imm[5:3|8:6] rs2 10 C.SDSP (RV64/128)
                match (inst >> 13) & 0x7 {
                    5 | 7 => {
                        // C.FSDSP
                        let imm5_3 = ((inst >> 10) & 0x7) as u32;
                        let imm8_6 = ((inst >> 7) & 0x7) as u32;
                        i.imm = ((imm8_6 << 6) | (imm5_3 << 3)) as i32;
                        i.rs1 = 2; // x2 is always the base pointer for CSS instructions
                    }
                    6 => {
                        // C.SWSP
                        let imm5_2 = ((inst >> 9) & 0xF) as u32;
                        let imm7_6 = ((inst >> 7) & 0x3) as u32;
                        i.imm = ((imm7_6 << 6) | (imm5_2 << 2)) as i32;
                        i.rs1 = 2; // x2 is always the base pointer for CSS instructions
                    }
                    _ => panic!(
                        "Invalid funct3={} for CSS at index={}",
                        (inst >> 13) & 0x7,
                        code_index
                    ),
                }

                i.rs2 = ((inst >> 2) & 0x1F) as u32;
            } else if i.t == "CIW" {
                // Format Meaning              |15 14 13 |12  11 10 9 8 7 6 5 |4 3 2 |1 0|
                // CIW    Wide Immediate       |funct3   |imm                 |rd′   |op |
                // Immediate is in format zimm[5:4|9:6|2|3]
                let imm5_4 = ((inst >> 11) & 0x3) as u32;
                let imm9_6 = ((inst >> 7) & 0xF) as u32;
                let imm2 = ((inst >> 6) & 0x1) as u32;
                let imm3 = ((inst >> 5) & 0x1) as u32;
                i.imm = ((imm9_6 << 6) | (imm5_4 << 4) | (imm3 << 3) | (imm2 << 2)) as i32;

                i.rd = Rvd::convert_compressed_reg_index(((inst >> 2) & 0x7) as u32);
                i.rs1 = 2; // x2 is always the source register for CIW instructions
            } else if i.t == "CL" {
                // Format Meaning              |15 14 13 |12  11 10 |9 8 7 |6 5 |4 3 2 |1 0|
                // CL     Load                 |funct3   |imm       |rs1′  |imm |rd′   |op |
                if inst_name == "c.lw" {
                    // Immediate is in format imm[5:3], imm[2|6]
                    let imm5_3 = ((inst >> 10) & 0x7) as u32;
                    let imm2 = ((inst >> 6) & 0x1) as u32;
                    let imm6 = ((inst >> 5) & 0x1) as u32;
                    i.imm = ((imm6 << 6) | (imm5_3 << 3) | (imm2 << 2)) as i32;
                } else {
                    // Immediate is in format imm[5:3], imm[7:6]
                    let imm5_3 = ((inst >> 10) & 0x7) as u32;
                    let imm7_6 = ((inst >> 5) & 0x3) as u32;
                    i.imm = ((imm7_6 << 6) | (imm5_3 << 3)) as i32;
                }
                i.rd = Rvd::convert_compressed_reg_index(((inst >> 2) & 0x7) as u32);
                i.rs1 = Rvd::convert_compressed_reg_index(((inst >> 7) & 0x7) as u32);
            } else if i.t == "CS" {
                // Format Meaning              |15 14 13 |12  11 10 |9 8 7 |6 5 |4 3 2 |1 0|
                // CS     Store                |funct3   |imm       |rs1′  |imm |rs2′  |op |
                if inst_name == "c.sw" {
                    // Immediate is in format imm[5:3], imm[2|6]
                    let imm5_3 = ((inst >> 10) & 0x7) as u32;
                    let imm2 = ((inst >> 6) & 0x1) as u32;
                    let imm6 = ((inst >> 5) & 0x1) as u32;
                    i.imm = ((imm6 << 6) | (imm5_3 << 3) | (imm2 << 2)) as i32;
                } else {
                    // Immediate is in format imm[5:3], imm[7:6]
                    let imm5_3 = ((inst >> 10) & 0x7) as u32;
                    let imm7_6 = ((inst >> 5) & 0x3) as u32;
                    i.imm = ((imm7_6 << 6) | (imm5_3 << 3)) as i32;
                }
                i.rs1 = Rvd::convert_compressed_reg_index(((inst >> 7) & 0x7) as u32);
                i.rs2 = Rvd::convert_compressed_reg_index(((inst >> 2) & 0x7) as u32);
            } else if i.t == "CA" {
                // Format Meaning              |15 14 13 12  11 10 |9 8 7   |6 5 |4 3 2 |1 0|
                // CA     Arithmetic           |funct6             |rd'/rs1'|fun2|rs2′  |op |
                i.rd = Rvd::convert_compressed_reg_index(((inst >> 7) & 0x7) as u32);
                i.rs1 = i.rd;
                i.rs2 = Rvd::convert_compressed_reg_index(((inst >> 2) & 0x7) as u32);
            } else if i.t == "CB" {
                // Format Meaning              |15 14 13 |12  11 10 |9 8 7 |6 5 4 3 2 |1 0|
                // CB     Branch               |funct3   |offset    |rs1′  |offset    |op |
                // Offset is in format offset[8|4:3] and offset[7:6|2:1|5]
                if inst_name == "c.andi" {
                    let imm5 = ((inst >> 12) & 0x1) as u32;
                    let imm4_0 = ((inst >> 2) & 0x1F) as u32;
                    i.imm = signext((imm5 << 5) | imm4_0, 6);
                    i.rd = Rvd::convert_compressed_reg_index(((inst >> 7) & 0x7) as u32);
                    i.rs1 = i.rd;
                    if i.rd == 0 {
                        panic!("Invalid use of rd=0 in c.andi at index={code_index}");
                    }
                } else if inst_name == "c.srli" {
                    let imm5 = ((inst >> 12) & 0x1) as u32;
                    let imm4_0 = ((inst >> 2) & 0x1F) as u32;
                    i.imm = ((imm5 << 5) | imm4_0) as i32;
                    i.rd = Rvd::convert_compressed_reg_index(((inst >> 7) & 0x7) as u32);
                    i.rs1 = i.rd;
                    if i.rd == 0 {
                        // This is a hint and must not be executed
                        i.inst = "c.nop".to_string(); // Change to c.nop
                    }
                } else {
                    let offset8 = ((inst >> 12) & 0x1) as u32;
                    let offset4_3 = ((inst >> 10) & 0x3) as u32;
                    let offset7_6 = ((inst >> 5) & 0x3) as u32;
                    let offset2_1 = ((inst >> 3) & 0x3) as u32;
                    let offset5 = ((inst >> 2) & 0x1) as u32;
                    let offset = (offset8 << 8)
                        | (offset7_6 << 6)
                        | (offset5 << 5)
                        | (offset4_3 << 3)
                        | (offset2_1 << 1);
                    i.imm = signext(offset, 9);
                    i.rs1 = Rvd::convert_compressed_reg_index(((inst >> 7) & 0x7) as u32);
                }
            } else if i.t == "CJ" {
                // Format Meaning              |15 14 13 |12  11 10 9 8 7 6 5 4 3 2 |1 0|
                // CJ     Jump                 |funct3   |jump target               |op |
                // Offset format is offset[11|4|9:8|10|6|7|3:1|5]
                let offset11 = ((inst >> 12) & 0x1) as u32;
                let offset4 = ((inst >> 11) & 0x1) as u32;
                let offset9_8 = ((inst >> 9) & 0x3) as u32;
                let offset10 = ((inst >> 8) & 0x1) as u32;
                let offset6 = ((inst >> 7) & 0x1) as u32;
                let offset7 = ((inst >> 6) & 0x1) as u32;
                let offset3_1 = ((inst >> 3) & 0x7) as u32;
                let offset5 = ((inst >> 2) & 0x1) as u32;
                let offset = (offset11 << 11)
                    | (offset10 << 10)
                    | (offset9_8 << 8)
                    | (offset7 << 7)
                    | (offset6 << 6)
                    | (offset5 << 5)
                    | (offset4 << 4)
                    | (offset3_1 << 1);
                i.imm = signext(offset, 12);
            } else {
                panic!("Invalid i.t={} at index={}", i.t, code_index);
            }
            insts.push(i);
        }
    }
    insts
}
