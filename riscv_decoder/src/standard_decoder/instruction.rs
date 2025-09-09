use std::fmt;

/// Instruction denotes the list of uncompressed instructions that
/// the decoder supports, ie those that can be outputted by the
/// standard decoder.
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Instruction {
    // -- RV32I Base Integer Instructions

    // R-type (register-register)
    ADD { rd: u8, rs1: u8, rs2: u8 },
    SUB { rd: u8, rs1: u8, rs2: u8 },
    SLL { rd: u8, rs1: u8, rs2: u8 },
    SLT { rd: u8, rs1: u8, rs2: u8 },
    SLTU { rd: u8, rs1: u8, rs2: u8 },
    XOR { rd: u8, rs1: u8, rs2: u8 },
    SRL { rd: u8, rs1: u8, rs2: u8 },
    SRA { rd: u8, rs1: u8, rs2: u8 },
    OR { rd: u8, rs1: u8, rs2: u8 },
    AND { rd: u8, rs1: u8, rs2: u8 },

    // I-type (immediate + loads + jalr + system)
    LB { rd: u8, rs1: u8, offset: i32 },
    LH { rd: u8, rs1: u8, offset: i32 },
    LW { rd: u8, rs1: u8, offset: i32 },
    LBU { rd: u8, rs1: u8, offset: i32 },
    LHU { rd: u8, rs1: u8, offset: i32 },
    ADDI { rd: u8, rs1: u8, imm: i32 },
    SLTI { rd: u8, rs1: u8, imm: i32 },
    SLTIU { rd: u8, rs1: u8, imm: i32 },
    XORI { rd: u8, rs1: u8, imm: i32 },
    ORI { rd: u8, rs1: u8, imm: i32 },
    ANDI { rd: u8, rs1: u8, imm: i32 },
    SLLI { rd: u8, rs1: u8, shamt: u8 },
    SRLI { rd: u8, rs1: u8, shamt: u8 },
    SRAI { rd: u8, rs1: u8, shamt: u8 },
    JALR { rd: u8, rs1: u8, offset: i32 },
    ECALL,
    EBREAK,

    // S-type (stores)
    SB { rs1: u8, rs2: u8, offset: i32 },
    SH { rs1: u8, rs2: u8, offset: i32 },
    SW { rs1: u8, rs2: u8, offset: i32 },

    // B-type (conditional branches)
    BEQ { rs1: u8, rs2: u8, offset: i32 },
    BNE { rs1: u8, rs2: u8, offset: i32 },
    BLT { rs1: u8, rs2: u8, offset: i32 },
    BGE { rs1: u8, rs2: u8, offset: i32 },
    BLTU { rs1: u8, rs2: u8, offset: i32 },
    BGEU { rs1: u8, rs2: u8, offset: i32 },

    // U-type (upper immediates)
    LUI { rd: u8, imm: i32 },
    AUIPC { rd: u8, imm: i32 },

    // J-type (unconditional jump)
    JAL { rd: u8, offset: i32 },

    // FENCE (misc-mem)
    FENCE { pred: u8, succ: u8 },

    // -- RV64I Extensions

    // I-type (loads + word-immediate ops)
    LD { rd: u8, rs1: u8, offset: i32 },
    LWU { rd: u8, rs1: u8, offset: i32 },
    ADDIW { rd: u8, rs1: u8, imm: i32 },
    SLLIW { rd: u8, rs1: u8, shamt: u8 },
    SRLIW { rd: u8, rs1: u8, shamt: u8 },
    SRAIW { rd: u8, rs1: u8, shamt: u8 },

    // R-type (word register operations)
    ADDW { rd: u8, rs1: u8, rs2: u8 },
    SUBW { rd: u8, rs1: u8, rs2: u8 },
    SLLW { rd: u8, rs1: u8, rs2: u8 },
    SRLW { rd: u8, rs1: u8, rs2: u8 },
    SRAW { rd: u8, rs1: u8, rs2: u8 },

    // S-type (stores)
    SD { rs1: u8, rs2: u8, offset: i32 },

    // -- RV32M/RV64M Multiply Extension
    //
    // All instructions in the `M` extension ar
    // the R-type
    MUL { rd: u8, rs1: u8, rs2: u8 },
    MULH { rd: u8, rs1: u8, rs2: u8 },
    MULHSU { rd: u8, rs1: u8, rs2: u8 },
    MULHU { rd: u8, rs1: u8, rs2: u8 },
    DIV { rd: u8, rs1: u8, rs2: u8 },
    DIVU { rd: u8, rs1: u8, rs2: u8 },
    REM { rd: u8, rs1: u8, rs2: u8 },
    REMU { rd: u8, rs1: u8, rs2: u8 },
    MULW { rd: u8, rs1: u8, rs2: u8 },
    DIVW { rd: u8, rs1: u8, rs2: u8 },
    DIVUW { rd: u8, rs1: u8, rs2: u8 },
    REMW { rd: u8, rs1: u8, rs2: u8 },
    REMUW { rd: u8, rs1: u8, rs2: u8 },

    // -- RV32A/RV64A Atomic Extension
    //
    // All instructions in the Atomic extension are the
    // A-type.

    // Load-reserved/store-conditional
    LR_W { rd: u8, rs1: u8, aq: bool, rl: bool },
    SC_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    LR_D { rd: u8, rs1: u8, aq: bool, rl: bool },
    SC_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },

    // Atomic memory operations (word)
    AMOSWAP_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOADD_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOXOR_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOAND_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOOR_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOMIN_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOMAX_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOMINU_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOMAXU_W { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },

    // Atomic memory operations (doubleword)
    AMOSWAP_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOADD_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOXOR_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOAND_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOOR_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOMIN_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOMAX_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOMINU_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AMOMAXU_D { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },

    // -- Zicsr Extension

    // All instructions in the Zicsr extension are the
    // I-type.
    CSRRW { rd: u8, rs1: u8, csr: u16 },
    CSRRS { rd: u8, rs1: u8, csr: u16 },
    CSRRC { rd: u8, rs1: u8, csr: u16 },
    CSRRWI { rd: u8, uimm: u8, csr: u16 },
    CSRRSI { rd: u8, uimm: u8, csr: u16 },
    CSRRCI { rd: u8, uimm: u8, csr: u16 },

    // -- Zifencei Extension
    FENCE_I,

    // Special instructions
    ILLEGAL,
}

impl Instruction {
    /// Returns the size of the instruction in bytes
    ///
    /// Note: uncompressed RISCV instructions have a fixed size,
    /// regardless of the instruction
    pub const fn size() -> usize {
        4
    }

    /// Get the mnemonic string for this instruction
    pub fn mnemonic(&self) -> &'static str {
        match self {
            Instruction::LB { .. } => "lb",
            Instruction::LH { .. } => "lh",
            Instruction::LW { .. } => "lw",
            Instruction::LBU { .. } => "lbu",
            Instruction::LHU { .. } => "lhu",
            Instruction::LD { .. } => "ld",
            Instruction::LWU { .. } => "lwu",

            Instruction::SB { .. } => "sb",
            Instruction::SH { .. } => "sh",
            Instruction::SW { .. } => "sw",
            Instruction::SD { .. } => "sd",

            Instruction::ADDI { .. } => "addi",
            Instruction::SLTI { .. } => "slti",
            Instruction::SLTIU { .. } => "sltiu",
            Instruction::XORI { .. } => "xori",
            Instruction::ORI { .. } => "ori",
            Instruction::ANDI { .. } => "andi",
            Instruction::SLLI { .. } => "slli",
            Instruction::SRLI { .. } => "srli",
            Instruction::SRAI { .. } => "srai",

            Instruction::ADD { .. } => "add",
            Instruction::SUB { .. } => "sub",
            Instruction::SLL { .. } => "sll",
            Instruction::SLT { .. } => "slt",
            Instruction::SLTU { .. } => "sltu",
            Instruction::XOR { .. } => "xor",
            Instruction::SRL { .. } => "srl",
            Instruction::SRA { .. } => "sra",
            Instruction::OR { .. } => "or",
            Instruction::AND { .. } => "and",

            Instruction::LUI { .. } => "lui",
            Instruction::AUIPC { .. } => "auipc",

            Instruction::BEQ { .. } => "beq",
            Instruction::BNE { .. } => "bne",
            Instruction::BLT { .. } => "blt",
            Instruction::BGE { .. } => "bge",
            Instruction::BLTU { .. } => "bltu",
            Instruction::BGEU { .. } => "bgeu",

            Instruction::JAL { .. } => "jal",
            Instruction::JALR { .. } => "jalr",

            Instruction::ADDIW { .. } => "addiw",
            Instruction::SLLIW { .. } => "slliw",
            Instruction::SRLIW { .. } => "srliw",
            Instruction::SRAIW { .. } => "sraiw",
            Instruction::ADDW { .. } => "addw",
            Instruction::SUBW { .. } => "subw",
            Instruction::SLLW { .. } => "sllw",
            Instruction::SRLW { .. } => "srlw",
            Instruction::SRAW { .. } => "sraw",

            Instruction::MUL { .. } => "mul",
            Instruction::MULH { .. } => "mulh",
            Instruction::MULHSU { .. } => "mulhsu",
            Instruction::MULHU { .. } => "mulhu",
            Instruction::DIV { .. } => "div",
            Instruction::DIVU { .. } => "divu",
            Instruction::REM { .. } => "rem",
            Instruction::REMU { .. } => "remu",
            Instruction::MULW { .. } => "mulw",
            Instruction::DIVW { .. } => "divw",
            Instruction::DIVUW { .. } => "divuw",
            Instruction::REMW { .. } => "remw",
            Instruction::REMUW { .. } => "remuw",

            Instruction::LR_W { .. } => "lr.w",
            Instruction::SC_W { .. } => "sc.w",
            Instruction::LR_D { .. } => "lr.d",
            Instruction::SC_D { .. } => "sc.d",
            Instruction::AMOSWAP_W { .. } => "amoswap.w",
            Instruction::AMOADD_W { .. } => "amoadd.w",
            Instruction::AMOXOR_W { .. } => "amoxor.w",
            Instruction::AMOAND_W { .. } => "amoand.w",
            Instruction::AMOOR_W { .. } => "amoor.w",
            Instruction::AMOMIN_W { .. } => "amomin.w",
            Instruction::AMOMAX_W { .. } => "amomax.w",
            Instruction::AMOMINU_W { .. } => "amominu.w",
            Instruction::AMOMAXU_W { .. } => "amomaxu.w",
            Instruction::AMOSWAP_D { .. } => "amoswap.d",
            Instruction::AMOADD_D { .. } => "amoadd.d",
            Instruction::AMOXOR_D { .. } => "amoxor.d",
            Instruction::AMOAND_D { .. } => "amoand.d",
            Instruction::AMOOR_D { .. } => "amoor.d",
            Instruction::AMOMIN_D { .. } => "amomin.d",
            Instruction::AMOMAX_D { .. } => "amomax.d",
            Instruction::AMOMINU_D { .. } => "amominu.d",
            Instruction::AMOMAXU_D { .. } => "amomaxu.d",

            Instruction::ECALL => "ecall",
            Instruction::EBREAK => "ebreak",
            Instruction::FENCE { .. } => "fence",

            Instruction::CSRRW { .. } => "csrrw",
            Instruction::CSRRS { .. } => "csrrs",
            Instruction::CSRRC { .. } => "csrrc",
            Instruction::CSRRWI { .. } => "csrrwi",
            Instruction::CSRRSI { .. } => "csrrsi",
            Instruction::CSRRCI { .. } => "csrrci",

            Instruction::FENCE_I => "fence.i",

            Instruction::ILLEGAL => "illegal",
        }
    }

    /// Check if this is a NOP instruction
    ///
    /// TODO: NOP is a special instance of what the spec calls hint instructions.
    /// TODO: Add the other hint instructions.
    pub fn is_nop(&self) -> bool {
        matches!(self, Instruction::ADDI { rd: 0, rs1: 0, imm: 0 })
    }

    /// Check if this is an illegal instruction
    pub fn is_illegal(&self) -> bool {
        matches!(self, Instruction::ILLEGAL)
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::ADDI { rd, rs1, imm } => write!(f, "addi x{}, x{}, {}", rd, rs1, imm),
            Instruction::ADD { rd, rs1, rs2 } => write!(f, "add x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::LW { rd, rs1, offset } => write!(f, "lw x{}, {}(x{})", rd, offset, rs1),
            Instruction::SW { rs1, rs2, offset } => write!(f, "sw x{}, {}(x{})", rs2, offset, rs1),
            Instruction::BEQ { rs1, rs2, offset } => {
                write!(f, "beq x{}, x{}, {}", rs1, rs2, offset)
            }
            Instruction::JAL { rd, offset } => write!(f, "jal x{}, {}", rd, offset),
            Instruction::LUI { rd, imm } => write!(f, "lui x{}, 0x{:x}", rd, imm),

            Instruction::AMOSWAP_W { rd, rs1, rs2, aq, rl } => {
                let suffix = match (aq, rl) {
                    (true, true) => ".aqrl",
                    (true, false) => ".aq",
                    (false, true) => ".rl",
                    (false, false) => "",
                };
                write!(f, "amoswap.w{} x{}, x{}, (x{})", suffix, rd, rs2, rs1)
            }

            Instruction::ECALL => write!(f, "ecall"),
            Instruction::EBREAK => write!(f, "ebreak"),
            Instruction::CSRRW { rd, rs1, csr } => {
                write!(f, "csrrw x{}, 0x{:x}, x{}", rd, csr, rs1)
            }

            Instruction::ILLEGAL => write!(f, "illegal"),

            _ => write!(f, "{}", self.mnemonic()),
        }
    }
}

impl Instruction {
    /// Create a NOP instruction (addi x0, x0, 0)
    pub fn nop() -> Self {
        Instruction::ADDI { rd: 0, rs1: 0, imm: 0 }
    }

    /// Create an illegal instruction
    pub fn illegal() -> Self {
        Instruction::ILLEGAL
    }
}
