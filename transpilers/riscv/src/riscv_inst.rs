//! RISC-V instruction definition
//!
//! Contains RISCV instruction data, split by functionality.
//! RISC-V comprises of a base user-level 32-bit integer instruction set. Called RV32I, it includes
//! 47 instructions, which can be grouped into six types:
//! * R-type: register-register
//! * I-type: short immediates and loads
//! * S-type: stores
//! * B-type: conditional branches, a variation of S-type
//! * U-type: long immediates
//! * J-type: unconditional jumps, a variation of U-type
//!
//! RV32I instruction formats showing immediate variants:
//! ```text
//!  31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13 12 11 10 09 08 07 06 05 04 03 02 01 00
//! |      funct7        |      rs2     |      rs1     | funct3 |      rd      |       opcode       | R-type
//! |               imm[11:0]           |      rs1     | funct3 |      rd      |       opcode       | I-type
//! |     imm[11:5]      |      rs2     |      rs1     | funct3 |   imm[4:0]   |       opcode       | S-type
//! |12|    imm[10:5]    |      rs2     |      rs1     | funct3 |imm[4:1]   |11|       opcode       | B-type
//! |                         imm[31:12]                        |      rd      |       opcode       | U-type
//! |20|           imm[10:1]         |11|      imm[19:12]       |      rd      |       opcode       | J-type
//! ```
//! RV32I has x0 register hardwired to constant 0, plus x1-x31 general purpose registers.
//! All registers are 32 bits wide but in RV64I they become 64 bits wide.
//! RV32I is a load-store architecture. This means that only load and store instructions access
//! memory; arithmetic operations use only the registers.
//! User space is 32-bit byte addressable and little endian.
//!
//! Correspondingly, RV64I is for 64-bit address space and RV128I is for 128-bit address space.  The
//! need for RV128I is debatable and its specification is evolving. We also have RV32E for embedded
//! systems. RV32E has only 16 32-bit registers and makes the counters of RV32I optional.
//!
//! See <https://devopedia.org/risc-v-instruction-sets>

use crate::{RiscvInstName, RiscvInstType};

/// RISC-V instruction data
#[derive(Default, Debug, Clone)]
pub struct RiscvInst {
    /// Instruction ROM address, i.e. program counter value
    pub rom_address: u64,

    /// Original instruction content (32 bits)
    pub rvinst: u32,

    /// Instruction type
    pub inst_type: RiscvInstType,

    /// Instruction mnemonic
    pub inst_name: RiscvInstName,

    // Instruction fields
    pub funct2: u32,
    pub funct3: u32,
    pub funct5: u32,
    pub funct6: u32,
    pub funct7: u32,
    pub rd: u32,
    pub rs1: u32,
    pub rs2: u32,
    pub rs3: u32,
    pub imm: i32,
    pub imme: u32,
    pub aq: u32,
    pub rl: u32,
    pub csr: u32,
    pub pred: u32,
    pub succ: u32,
}

impl RiscvInst {
    /// Creates a NOP instruction (ADDI x0, x0, 0)
    pub fn nop(rvinst: u32, rom_address: u64) -> Self {
        Self {
            rvinst,
            rom_address,
            inst_type: RiscvInstType::I,
            inst_name: RiscvInstName::Addi,
            rd: 0,
            rs1: 0,
            rs2: 0,
            imm: 0,
            ..Default::default()
        }
    }

    /// Creates a HALT instruction
    pub fn c_halt(rvinst: u32, rom_address: u64) -> Self {
        Self {
            rvinst,
            rom_address,
            inst_type: RiscvInstType::Cinvalid,
            inst_name: RiscvInstName::CHalt,
            rd: 0,
            rs1: 0,
            rs2: 0,
            imm: 0,
            ..Default::default()
        }
    }

    /// Creates a human-readable string containing RISCV data fields that are non-zero
    pub fn to_text(&self) -> String {
        // Preallocate a string with a reasonable capacity to avoid multiple allocations
        let mut s = String::with_capacity(256);

        // Add instruction type
        s.push_str("type=");
        s.push_str(self.inst_type.as_str());

        // Add instruction name
        s.push_str(" name=");
        s.push_str(self.inst_name.as_str());

        // Add instruction fields if they are non-zero
        if self.rvinst != 0 {
            s.push_str(" rvinst=");
            s.push_str(&self.rvinst.to_string());
        }
        if self.funct3 != 0 {
            s.push_str(" funct3=");
            s.push_str(&self.funct3.to_string());
        }
        if self.funct5 != 0 {
            s.push_str(" funct5=");
            s.push_str(&self.funct5.to_string());
        }
        if self.funct6 != 0 {
            s.push_str(" funct6=");
            s.push_str(&self.funct6.to_string());
        }
        if self.funct7 != 0 {
            s.push_str(" funct7=");
            s.push_str(&self.funct7.to_string());
        }
        if self.rd != 0 {
            s.push_str(" rd=");
            s.push_str(&self.rd.to_string());
        }
        if self.rs1 != 0 {
            s.push_str(" rs1=");
            s.push_str(&self.rs1.to_string());
        }
        if self.rs2 != 0 {
            s.push_str(" rs2=");
            s.push_str(&self.rs2.to_string());
        }
        if self.imm != 0 {
            s.push_str(" imm=");
            s.push_str(&self.imm.to_string());
        }
        if self.imme != 0 {
            s.push_str(" imme=");
            s.push_str(&self.imme.to_string());
        }
        if self.aq != 0 {
            s.push_str(" aq=");
            s.push_str(&self.aq.to_string());
        }
        if self.rl != 0 {
            s.push_str(" rl=");
            s.push_str(&self.rl.to_string());
        }
        if self.csr != 0 {
            s.push_str(" csr=");
            s.push_str(&self.csr.to_string());
        }
        if self.pred != 0 {
            s.push_str(" pred=");
            s.push_str(&self.pred.to_string());
        }
        if self.succ != 0 {
            s.push_str(" succ=");
            s.push_str(&self.succ.to_string());
        }
        s
    }
}
