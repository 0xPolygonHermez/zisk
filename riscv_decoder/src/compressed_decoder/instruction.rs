use crate::standard_decoder::Instruction as StandardInstruction;

/// Instruction denotes the list of compressed (16-bit) instructions that
/// the decoder supports, ie those that can be outputted by the
/// compressed decoder.
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Instruction {
    // Compressed Instructions (RVC)

    // CIW (wide immediate)
    C_ADDI4SPN { rd: u8, imm: u16 },

    // CL (loads)
    C_LW { rd: u8, rs1: u8, offset: u8 },
    C_LD { rd: u8, rs1: u8, offset: u8 },

    // CS (stores) and CSS (stack stores)
    C_SW { rs1: u8, rs2: u8, offset: u8 },
    C_SD { rs1: u8, rs2: u8, offset: u8 },
    C_SWSP { rs2: u8, offset: u8 },
    C_SDSP { rs2: u8, offset: u8 },

    // CI (immediates and moves, sp-relative loads)
    C_NOP,
    C_ADDI { rd: u8, imm: i8 },
    C_ADDIW { rd: u8, imm: i8 },
    C_LI { rd: u8, imm: i8 },
    C_ADDI16SP { imm: i16 },
    C_LUI { rd: u8, imm: i32 },
    C_SRLI { rd: u8, shamt: u8 },
    C_SRAI { rd: u8, shamt: u8 },
    C_ANDI { rd: u8, imm: i8 },
    C_SLLI { rd: u8, shamt: u8 },
    C_LWSP { rd: u8, offset: u8 },
    C_LDSP { rd: u8, offset: u8 },

    // CA (arith on compressed regs)
    C_SUB { rd: u8, rs2: u8 },
    C_XOR { rd: u8, rs2: u8 },
    C_OR { rd: u8, rs2: u8 },
    C_AND { rd: u8, rs2: u8 },
    C_SUBW { rd: u8, rs2: u8 },
    C_ADDW { rd: u8, rs2: u8 },

    // CB (branches)
    C_BEQZ { rs1: u8, offset: i8 },
    C_BNEZ { rs1: u8, offset: i8 },

    // CJ (jumps)
    C_J { offset: i16 },
    // TODO: Need to double check the specs on this
    // TODO: This only works for RV32 - since it overlaps with C_ADDIW on RV64
    // TODO note, that we cannot have rv32c and rv64c on at the same time
    // Spec: "In addition, RV32C includes a compressed jump and link instruction
    // to compress short-range subroutine calls, where the same opcode is used to compress ADDIW for RV64C and RV128C.""
    // C_JAL { offset: i16 },

    // CR (register-register & control)
    C_JR { rs1: u8 },
    C_MV { rd: u8, rs2: u8 },
    C_EBREAK,
    C_JALR { rs1: u8 },
    C_ADD { rd: u8, rs2: u8 },

    // Compressed illegal instruction (c.unimp)
    C_ILLEGAL,
}

impl Instruction {
    /// Returns the size of the instruction in bytes
    ///
    /// Note: compressed RISCV instructions have a fixed size,
    /// regardless of the instruction
    pub const fn size() -> usize {
        2
    }

    /// Get the mnemonic string for this compressed instruction
    pub fn mnemonic(&self) -> &'static str {
        match self {
            Instruction::C_ADDI4SPN { .. } => "c.addi4spn",
            Instruction::C_LW { .. } => "c.lw",
            Instruction::C_LD { .. } => "c.ld",
            Instruction::C_SW { .. } => "c.sw",
            Instruction::C_SD { .. } => "c.sd",
            Instruction::C_NOP => "c.nop",
            Instruction::C_ADDI { .. } => "c.addi",
            Instruction::C_ADDIW { .. } => "c.addiw",
            Instruction::C_LI { .. } => "c.li",
            Instruction::C_ADDI16SP { .. } => "c.addi16sp",
            Instruction::C_LUI { .. } => "c.lui",
            Instruction::C_SRLI { .. } => "c.srli",
            Instruction::C_SRAI { .. } => "c.srai",
            Instruction::C_ANDI { .. } => "c.andi",
            Instruction::C_SLLI { .. } => "c.slli",
            Instruction::C_LWSP { .. } => "c.lwsp",
            Instruction::C_LDSP { .. } => "c.ldsp",
            Instruction::C_SUB { .. } => "c.sub",
            Instruction::C_XOR { .. } => "c.xor",
            Instruction::C_OR { .. } => "c.or",
            Instruction::C_AND { .. } => "c.and",
            Instruction::C_SUBW { .. } => "c.subw",
            Instruction::C_ADDW { .. } => "c.addw",
            Instruction::C_BEQZ { .. } => "c.beqz",
            Instruction::C_BNEZ { .. } => "c.bnez",
            Instruction::C_J { .. } => "c.j",
            Instruction::C_JR { .. } => "c.jr",
            Instruction::C_MV { .. } => "c.mv",
            Instruction::C_EBREAK => "c.ebreak",
            Instruction::C_JALR { .. } => "c.jalr",
            Instruction::C_ADD { .. } => "c.add",
            Instruction::C_SWSP { .. } => "c.swsp",
            Instruction::C_SDSP { .. } => "c.sdsp",
            Instruction::C_ILLEGAL => "c.unimp",
        }
    }
}

impl From<Instruction> for StandardInstruction {
    fn from(value: Instruction) -> Self {
        match value {
            // Stack pointer operations
            Instruction::C_ADDI4SPN { rd, imm } => {
                // c.addi4spn rd′, nzuimm -> addi rd, x2, nzuimm
                // Adds non-zero unsigned immediate to stack pointer, writes result to rd
                // x2 is stack pointer which is why we set rs1 to 2
                StandardInstruction::ADDI { rd, rs1: 2, imm: imm as i32 }
            }

            // Loads
            Instruction::C_LW { rd, rs1, offset } => {
                // c.lw rd′, offset(rs1′) -> lw rd, offset(rs1)
                // Load word from rs1 + offset into rd
                StandardInstruction::LW { rd, rs1, offset: offset as i32 }
            }
            Instruction::C_LD { rd, rs1, offset } => {
                // c.ld rd′, offset(rs1′) -> ld rd, offset(rs1)
                // Load doubleword from rs1 + offset into rd
                StandardInstruction::LD { rd, rs1, offset: offset as i32 }
            }
            Instruction::C_LWSP { rd, offset } => {
                // c.lwsp rd, offset(sp) -> lw rd, offset(x2)
                // Load word from stack pointer + offset into rd
                StandardInstruction::LW { rd, rs1: 2, offset: offset as i32 }
            }
            Instruction::C_LDSP { rd, offset } => {
                // c.ldsp rd, offset(sp) -> ld rd, offset(x2)
                // Load doubleword from stack pointer + offset into rd
                StandardInstruction::LD { rd, rs1: 2, offset: offset as i32 }
            }

            // Stores
            Instruction::C_SW { rs1, rs2, offset } => {
                // c.sw rs2′, offset(rs1′) -> sw rs2, offset(rs1)
                // Store word from rs2 into memory at rs1 + offset
                StandardInstruction::SW { rs1, rs2, offset: offset as i32 }
            }
            Instruction::C_SD { rs1, rs2, offset } => {
                // c.sd rs2′, offset(rs1′) -> sd rs2, offset(rs1)
                // Store doubleword from rs2 into memory at rs1 + offset
                StandardInstruction::SD { rs1, rs2, offset: offset as i32 }
            }
            Instruction::C_SWSP { rs2, offset } => {
                // c.swsp rs2, offset(sp) → sw rs2, offset(x2)
                // Store word from rs2 into memory at stack pointer + offset
                StandardInstruction::SW { rs1: 2, rs2, offset: offset as i32 }
            }
            Instruction::C_SDSP { rs2, offset } => {
                // c.sdsp rs2, offset(sp) → sd rs2, offset(x2)
                // Store doubleword from rs2 into memory at stack pointer + offset
                StandardInstruction::SD { rs1: 2, rs2, offset: offset as i32 }
            }

            // Immediate operations
            Instruction::C_NOP => {
                // c.nop -> addi x0, x0, 0
                StandardInstruction::ADDI { rd: 0, rs1: 0, imm: 0 }
            }
            Instruction::C_ADDI { rd, imm } => {
                // c.addi rd, imm -> addi rd, rd, imm
                StandardInstruction::ADDI { rd, rs1: rd, imm: imm as i32 }
            }
            Instruction::C_ADDIW { rd, imm } => {
                // c.addiw rd, imm -> addiw rd, rd, imm
                StandardInstruction::ADDIW { rd, rs1: rd, imm: imm as i32 }
            }
            Instruction::C_LI { rd, imm } => {
                // c.li rd, imm -> addi rd, x0, imm
                StandardInstruction::ADDI { rd, rs1: 0, imm: imm as i32 }
            }
            Instruction::C_ADDI16SP { imm } => {
                // c.addi16sp imm -> addi x2, x2, imm
                StandardInstruction::ADDI { rd: 2, rs1: 2, imm: imm as i32 }
            }
            Instruction::C_LUI { rd, imm } => {
                // c.lui rd, imm -> lui rd, imm
                StandardInstruction::LUI { rd, imm }
            }

            // Shift operations
            Instruction::C_SLLI { rd, shamt } => {
                // c.slli rd, shamt -> slli rd, rd, shamt
                StandardInstruction::SLLI { rd, rs1: rd, shamt }
            }
            Instruction::C_SRLI { rd, shamt } => {
                // c.srli rd', shamt -> srli rd', rd', shamt
                StandardInstruction::SRLI { rd, rs1: rd, shamt }
            }
            Instruction::C_SRAI { rd, shamt } => {
                // c.srai rd', shamt -> srai rd', rd', shamt
                StandardInstruction::SRAI { rd, rs1: rd, shamt }
            }
            Instruction::C_ANDI { rd, imm } => {
                // c.andi rd', imm -> andi rd', rd', imm
                StandardInstruction::ANDI { rd, rs1: rd, imm: imm as i32 }
            }

            // Arithmetic operations
            Instruction::C_SUB { rd, rs2 } => {
                // c.sub rd', rs2' -> sub rd', rd', rs2'
                StandardInstruction::SUB { rd, rs1: rd, rs2 }
            }
            Instruction::C_XOR { rd, rs2 } => {
                // c.xor rd', rs2' -> xor rd', rd', rs2'
                StandardInstruction::XOR { rd, rs1: rd, rs2 }
            }
            Instruction::C_OR { rd, rs2 } => {
                // c.or rd', rs2' -> or rd', rd', rs2'
                StandardInstruction::OR { rd, rs1: rd, rs2 }
            }
            Instruction::C_AND { rd, rs2 } => {
                // c.and rd', rs2' -> and rd', rd', rs2'
                StandardInstruction::AND { rd, rs1: rd, rs2 }
            }
            Instruction::C_SUBW { rd, rs2 } => {
                // c.subw rd', rs2' -> subw rd', rd', rs2'
                StandardInstruction::SUBW { rd, rs1: rd, rs2 }
            }
            Instruction::C_ADDW { rd, rs2 } => {
                // c.addw rd', rs2' -> addw rd', rd', rs2'
                StandardInstruction::ADDW { rd, rs1: rd, rs2 }
            }

            // Control flow
            Instruction::C_J { offset } => {
                // c.j offset -> jal x0, offset
                StandardInstruction::JAL { rd: 0, offset: offset as i32 }
            }
            Instruction::C_BEQZ { rs1, offset } => {
                // c.beqz rs1', offset -> beq rs1', x0, offset
                StandardInstruction::BEQ { rs1, rs2: 0, offset: offset as i32 }
            }
            Instruction::C_BNEZ { rs1, offset } => {
                // c.bnez rs1', offset -> bne rs1', x0, offset
                StandardInstruction::BNE { rs1, rs2: 0, offset: offset as i32 }
            }
            Instruction::C_JR { rs1 } => {
                // c.jr rs1 -> jalr x0, 0(rs1)
                StandardInstruction::JALR { rd: 0, rs1, offset: 0 }
            }
            Instruction::C_JALR { rs1 } => {
                // c.jalr rs1 -> jalr x1, 0(rs1)
                StandardInstruction::JALR { rd: 1, rs1, offset: 0 }
            }
            Instruction::C_MV { rd, rs2 } => {
                // c.mv rd, rs2 -> add rd, x0, rs2
                StandardInstruction::ADD { rd, rs1: 0, rs2 }
            }
            Instruction::C_ADD { rd, rs2 } => {
                // c.add rd, rs2 -> add rd, rd, rs2
                StandardInstruction::ADD { rd, rs1: rd, rs2 }
            }

            Instruction::C_EBREAK => StandardInstruction::EBREAK,
            Instruction::C_ILLEGAL => StandardInstruction::ILLEGAL,
        }
    }
}
