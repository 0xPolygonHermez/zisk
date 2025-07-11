//! Parses a 32-bits RISC-V instruction

use crate::{RiscvInstruction, Rvd, RvdOperation};

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

/// Gets the RUSTC instruction in text and tree level, based on the RVD operation and 2 tree
/// branches indexes
fn getinst(op: &RvdOperation, i1: u32, i2: u32) -> (String, i32) {
    if !op.s.is_empty() {
        return (op.s.clone(), 0);
    }
    if !op.map.contains_key(&i1) {
        return (String::new(), -1);
    }
    if !op.map[&i1].s.is_empty() {
        return (op.map[&i1].s.clone(), 1);
    }
    if !op.map[&i1].map.contains_key(&i2) {
        return (String::new(), -1);
    }
    if !op.map[&i1].map[&i2].s.is_empty() {
        return (op.map[&i1].map[&i2].s.clone(), 2);
    }
    (String::new(), -1)
}

/// Interprets a buffer of 32-bits RICSV instructions into a vector of decoded RISCV instructions
/// split by field
pub fn riscv_interpreter(code: &[u32]) -> Vec<RiscvInstruction> {
    let mut insts = Vec::<RiscvInstruction>::new();

    // Build an RVD data tree
    let mut rvd = Rvd::new();
    rvd.init();

    // For every 32-bit instruction in the input code buffer
    let code_len = code.len();
    for (s, inst_ref) in code.iter().enumerate().take(code_len) {
        //println!("riscv_interpreter() s={}", s);

        // Get the RISCV instruction
        let inst = *inst_ref;

        // Ignore instructions that are zero
        if inst == 0 {
            //println!("riscv_interpreter() found inst=0 at position s={}", s);
            continue;
        }

        // Extract the opcode from the lower 7 bits of the RICSV instruction
        let opcode = inst & 0x7F;

        // Get the RVD info data for this opcode
        if !rvd.opcodes.contains_key(&opcode) {
            panic!("Invalid opcode={opcode}=0x{opcode:x} s={s}");
        }
        let inf = &rvd.opcodes[&opcode];

        // Create a RISCV instruction instance to be filled with data from the instruction and from
        // the RVD info data
        // Copy the original RISCV 32-bit instruction
        // Copy the instruction type
        let mut i = RiscvInstruction { rvinst: inst, t: inf.t.clone(), ..Default::default() };

        // Decode the rest of instruction fields based on the instruction type

        //  31 30 ... 21 20 19 ... 15 14 13 12 11 ... 07 06 05 04 03 02 01 00
        // |  imm[11:0]    |  rs1    | funct3 |   rd    |       opcode       | I-type
        if i.t == *"I" {
            i.funct3 = (inst & 0x7000) >> 12;
            let funct7 = (inst & 0xFC000000) >> 26;
            i.rd = (inst & 0xF80) >> 7;
            i.rs1 = (inst & 0xF8000) >> 15;
            i.imm = signext((inst & 0xFFF00000) >> 20, 12);
            let l: i32;
            (i.inst, l) = getinst(&inf.op, i.funct3, funct7);
            assert!(!i.inst.is_empty());
            if l == 2 {
                i.imm &= 0x3F;
                i.funct7 = funct7;
            }
        }
        //  31 30 ... 26 25 24 ... 20 19 ... 15 14 13 12 11 ... 07 06 05 04 03 02 01 00
        // |   funct7      |  rs2    |  rs1    | funct3 |   rd    |       opcode       | R-type
        else if i.t == *"R" {
            i.funct3 = (inst & 0x7000) >> 12;
            i.rd = (inst & 0xF80) >> 7;
            i.rs1 = (inst & 0xF8000) >> 15;
            i.rs2 = (inst & 0x1F00000) >> 20;
            i.funct7 = (inst & 0xFE000000) >> 25;
            (i.inst, _) = getinst(&inf.op, i.funct3, i.funct7);
            assert!(!i.inst.is_empty());
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
            (i.inst, _) = getinst(&inf.op, i.funct3, 0);
            assert!(!i.inst.is_empty());
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
            (i.inst, _) = getinst(&inf.op, i.funct3, 0);
            assert!(!i.inst.is_empty());
        }
        //  31 30 ... 13 12 11 10 09 08 07 06 05 04 03 02 01 00
        // |  imm[31:12]   |      rd      |        opcode      | U-type
        else if i.t == *"U" {
            i.rd = (inst & 0xF80) >> 7;
            i.imm = (((inst & 0xFFFFF000) >> 12) << 12) as i32;
            (i.inst, _) = getinst(&inf.op, 0, 0);
            assert!(!i.inst.is_empty());
        }
        //  31 30 29...22 21 20 19 18 ... 13 12 11 10 09 08 07 06 05 04 03 02 01 00
        // |20|  imm[10:1]  |11|  imm[19:12]   |      rd      |       opcode       | J-type
        else if i.t == *"J" {
            i.rd = (inst & 0xF80) >> 7;
            let imm20 = (inst & 0x80000000) >> 31;
            let imm10_1 = (inst & 0x7FE00000) >> 21;
            let imm11j = (inst & 0x100000) >> 20;
            let imm19_12 = (inst & 0xFF000) >> 12;
            i.imm = signext((imm20 << 20) | (imm19_12 << 12) | (imm11j << 11) | (imm10_1 << 1), 21);
            (i.inst, _) = getinst(&inf.op, 0, 0);
            assert!(!i.inst.is_empty());
        } else if i.t == *"A" {
            i.funct3 = (inst & 0x7000) >> 12;
            i.rd = (inst & 0xF80) >> 7;
            i.rs1 = (inst & 0xF8000) >> 15;
            i.rs2 = (inst & 0x1F00000) >> 20;
            i.funct5 = (inst & 0xF8000000) >> 27;
            i.aq = (inst & 0x4000000) >> 26;
            i.rl = (inst & 0x2000000) >> 24;
            (i.inst, _) = getinst(&inf.op, i.funct3, i.funct5);
            assert!(!i.inst.is_empty());
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
                    // throw new Error(`Invalid opcode: ${opcode} at line ${s}`);
                }
            } else {
                i.rd = (inst & 0xF80) >> 7;
                if (i.funct3 & 0x4) != 0 {
                    i.imme = (inst & 0xF8000) >> 15;
                } else {
                    i.rs1 = (inst & 0xF8000) >> 15;
                }
                i.csr = (inst & 0xFFF00000) >> 20;
                (i.inst, _) = getinst(&inf.op, i.funct3, 0);
                assert!(!i.inst.is_empty());
            }
        } else if i.t == *"F" {
            i.funct3 = (inst & 0x7000) >> 12;
            if i.funct3 == 0 {
                if (inst & 0xF00F8F80) != 0 {
                    panic!("Invalid opcode={opcode} at line s={s}");
                }
                i.pred = (inst & 0x0F000000) >> 24;
                i.succ = (inst & 0x00F00000) >> 20;
                i.inst = "fence".to_string();
            } else if i.funct3 == 1 {
                if (inst & 0xFFFF8F80) != 0 {
                    panic!("Invalid opcode={opcode} at line s={s}");
                }
                i.inst = "fence.i".to_string();
            } else {
                panic!("Invalid opcode={opcode} at line s={s}");
            }
        } else {
            panic!("Invalid i.t={} at line s={}", i.t, s);
        }
        insts.push(i);
    }
    insts
}
