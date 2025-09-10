//! 32-bit instruction decoder. This decoder assumes a specific set of instructions are enabled.
//! See `crate::target::Extension` for what these extensions are.

mod error;
mod instruction;
mod opcode;

pub use error::Error;
pub use instruction::Instruction;

use crate::standard_decoder::opcode::{InstructionFormat, Opcode};

/// Decode a u32 into a standard RISC-V instruction
///
/// Returns an Error if the instruction is not recognized or malformed.
///
/// Note: The instruction should not be compressed.
pub fn decode_standard_instruction(bits: u32) -> Result<Instruction, Error> {
    // Handle special case: all zeros = illegal
    if bits == 0 {
        return Ok(Instruction::illegal());
    }

    // Parse all instruction fields
    let encoded = EncodedInstruction::new(bits);

    // Check if parsed opcode is valid
    let opcode = Opcode::from_bits(encoded.opcode)
        .ok_or(Error::UnsupportedOpcode { opcode_bits: encoded.opcode })?;

    match opcode {
        Opcode::Load => decode_load_instruction(&encoded),
        Opcode::MiscMem => decode_fence_instruction(&encoded),
        Opcode::OpImm => decode_op_imm_instruction(&encoded),
        Opcode::OpImm32 => decode_op_imm_32_instruction(&encoded),
        Opcode::Op => decode_op_instruction(&encoded),
        Opcode::Op32 => decode_op_32_instruction(&encoded),
        Opcode::Auipc => decode_auipc_instruction(&encoded),
        Opcode::Store => decode_store_instruction(&encoded),
        Opcode::Branch => decode_branch_instruction(&encoded),
        Opcode::Jal => decode_jal_instruction(&encoded),
        Opcode::Jalr => decode_jalr_instruction(&encoded),
        Opcode::Lui => decode_lui_instruction(&encoded),
        Opcode::Amo => decode_amo_instruction(&encoded),
        Opcode::System => decode_system_instruction(&encoded),
    }
}

const MASK1: u32 = 0b1;
const MASK3: u32 = 0b111;
const MASK4: u32 = 0b1111;
const MASK5: u32 = 0b1_1111;
const MASK6: u32 = 0b11_1111;
const MASK7: u32 = 0b111_1111;
const MASK8: u32 = 0b1111_1111;
const MASK10: u32 = 0b11_1111_1111;
const MASK12: u32 = 0b1111_1111_1111;

/// EncodedInstruction holds all of the possible parsed fields from a 32-bit RISC-V instruction
///
/// This can be seen as a union of all of the instruction formats, the decoder then
/// picks the relevant fields based on the opcode, which greatly simplifies the decoding methods.
///
/// The nice thing about the instruction formats in RISCV is that the same fields are always in the same
/// bit positions. For example, the destination register is always in bit positions [11:7].
///
/// Note: This does mean that redundant work is being done, for example
/// `aq` is being extracted each time, when it is only relevant for atomic
/// instructions.
/// This redundant work however is acceptable because bitwise operations
/// are fast.
#[derive(Debug, Clone, PartialEq)]
struct EncodedInstruction {
    /// Original 32-bit instruction word
    pub raw: u32,

    /// Opcode field (bits [6:0])
    pub opcode: u8,

    /// Destination register (bits [11:7])
    pub rd: u8,

    /// Function code 3 (bits [14:12])
    pub funct3: u8,

    /// Source register 1 (bits [19:15])
    pub rs1: u8,

    /// Source register 2 (bits [24:20])
    pub rs2: u8,

    /// Function code 7 (bits [31:25])
    pub funct7: u8,

    /// I-type immediate (bits [31:20], sign-extended)
    pub i_immediate: i32,

    /// S-type immediate (split across bits [31:25] and [11:7], sign-extended)
    pub s_immediate: i32,

    /// B-type immediate (branch offset, sign-extended)
    pub b_immediate: i32,

    /// U-type immediate (bits [31:12], left-shifted by 12)
    pub u_immediate: i32,

    /// J-type immediate (jump offset, sign-extended)
    pub j_immediate: i32,

    /// CSR address (bits [31:20]) for system instructions
    pub csr: u16,

    /// Shift amount for RV32I (5-bit, bits [24:20])
    pub shamt32: u8,

    /// Shift amount for RV64I (6-bit, bits [25:20])
    pub shamt64: u8,

    /// Acquire bit (bit [26]) for atomic instructions
    pub aq: bool,

    /// Release bit (bit [25]) for atomic instructions  
    pub rl: bool,

    /// Function code 5 (bits [31:27]) for atomic instructions
    pub funct5: u8,

    /// Predecessor field (bits [27:24]) for fence instructions
    pub pred: u8,

    /// Successor field (bits [23:20]) for fence instructions
    pub succ: u8,

    /// FM field (bits [31:28]) for fence instructions
    pub fm: u8,
}

impl EncodedInstruction {
    /// Parse all possible fields from a 32-bit instruction
    pub fn new(raw: u32) -> Self {
        // Opcode is always the first 7 bits
        let opcode = (raw & MASK7) as u8;
        // `rd` is always the next 5 bits
        let rd = ((raw >> 7) & MASK5) as u8;
        // `funct3` is always the next 3 bits
        let funct3 = ((raw >> 12) & MASK3) as u8;
        // `rs1` is always the next 5 bits
        let rs1 = ((raw >> 15) & MASK5) as u8;
        // `rs2` is always the next 5 bits
        let rs2 = ((raw >> 20) & MASK5) as u8;
        // `funct7` is always the next 7 bits
        let funct7 = ((raw >> 25) & MASK7) as u8;

        // Extract all possible immediate formats
        let i_immediate = Self::extract_i_immediate(raw);
        let s_immediate = Self::extract_s_immediate(raw);
        let b_immediate = Self::extract_b_immediate(raw);
        let u_immediate = Self::extract_u_immediate(raw);
        let j_immediate = Self::extract_j_immediate(raw);

        // Extract other specialized fields
        let csr = ((raw >> 20) & MASK12) as u16; // 12-bit CSR address -- note, no sign extension here for csr address
        let shamt32 = ((raw >> 20) & MASK5) as u8; // 5-bit shift amount for RV32I
        let shamt64 = ((raw >> 20) & MASK6) as u8; // 6-bit shift amount for RV64I
        let aq = ((raw >> 26) & MASK1) != 0;
        let rl = ((raw >> 25) & MASK1) != 0;
        let funct5 = ((raw >> 27) & MASK5) as u8;
        let pred = ((raw >> 24) & MASK4) as u8;
        let succ = ((raw >> 20) & MASK4) as u8;
        let fm = ((raw >> 28) & MASK4) as u8;

        Self {
            raw,
            opcode,
            rd,
            funct3,
            rs1,
            rs2,
            funct7,
            i_immediate,
            s_immediate,
            b_immediate,
            u_immediate,
            j_immediate,
            csr,
            shamt32,
            shamt64,
            aq,
            rl,
            funct5,
            pred,
            succ,
            fm,
        }
    }

    /// Extract I-type immediate (12-bit, sign-extended)
    fn extract_i_immediate(raw: u32) -> i32 {
        let imm = (raw >> 20) & MASK12;

        // sign-extend from 12 bits
        ((imm as i32) << 20) >> 20
    }

    /// Extract S-type immediate (12-bit split, sign-extended)
    fn extract_s_immediate(raw: u32) -> i32 {
        let imm11_5 = ((raw >> 25) & MASK7) << 5;
        let imm4_0 = (raw >> 7) & MASK5;

        let imm = imm11_5 | imm4_0;

        // sign-extend from 12 bits
        ((imm as i32) << 20) >> 20
    }

    /// Extract B-type immediate (13-bit branch offset, sign-extended)
    fn extract_b_immediate(raw: u32) -> i32 {
        let imm12 = ((raw >> 31) & MASK1) << 12;
        let imm10_5 = ((raw >> 25) & MASK6) << 5;
        let imm4_1 = ((raw >> 8) & MASK4) << 1;
        let imm11 = ((raw >> 7) & MASK1) << 11;

        let imm = imm12 | imm11 | imm10_5 | imm4_1;

        // sign-extend from 13 bits
        ((imm as i32) << 19) >> 19
    }

    /// Extract U-type immediate (20-bit immediate value)
    fn extract_u_immediate(raw: u32) -> i32 {
        (raw >> 12) as i32
    }

    /// Extract J-type immediate (21-bit jump offset, sign-extended)  
    fn extract_j_immediate(raw: u32) -> i32 {
        let imm20 = ((raw >> 31) & MASK1) << 20;
        let imm10_1 = ((raw >> 21) & MASK10) << 1;
        let imm11 = ((raw >> 20) & MASK1) << 11;
        let imm19_12 = ((raw >> 12) & MASK8) << 12;

        let imm = imm20 | imm19_12 | imm11 | imm10_1;

        // sign-extend from 21 bits
        ((imm as i32) << 11) >> 11
    }

    /// Get the instruction format based on opcode
    /// TODO: Del this is only needed for Documentation and possibly tests
    /// TODO: so we can delete it and just have comments ontop of opcode for example
    /// TODO: This would mean we no longer need InstructionFormat struct
    pub fn format(&self) -> Option<InstructionFormat> {
        match Opcode::from_bits(self.opcode)? {
            Opcode::Op | Opcode::Op32 => Some(InstructionFormat::R),
            Opcode::Load
            | Opcode::OpImm
            | Opcode::OpImm32
            | Opcode::Jalr
            | Opcode::MiscMem
            | Opcode::System => Some(InstructionFormat::I),
            Opcode::Store => Some(InstructionFormat::S),
            Opcode::Branch => Some(InstructionFormat::B),
            Opcode::Lui | Opcode::Auipc => Some(InstructionFormat::U),
            Opcode::Jal => Some(InstructionFormat::J),
            Opcode::Amo => Some(InstructionFormat::R), // A-type uses R-type format base
        }
    }
}

/// Decode `LOAD` opcode instructions
///
/// Uses standard I-type format (see InstructionFormat::I)
fn decode_load_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let offset = encoded.i_immediate;

    match encoded.funct3 {
        0b000 => Ok(Instruction::LB { rd, rs1, offset }),
        0b001 => Ok(Instruction::LH { rd, rs1, offset }),
        0b010 => Ok(Instruction::LW { rd, rs1, offset }),
        0b011 => {
            // Requires RV64I
            Ok(Instruction::LD { rd, rs1, offset })
        }
        0b100 => Ok(Instruction::LBU { rd, rs1, offset }),
        0b101 => Ok(Instruction::LHU { rd, rs1, offset }),
        0b110 => {
            // Requires RV64I
            Ok(Instruction::LWU { rd, rs1, offset })
        }
        _ => Err(Error::InvalidFormat),
    }
}

/// Decode `FENCE` opcode instructions
///
/// Uses standard I-type format (see InstructionFormat::I)
///
/// The docs also note how fence specific information is encoded
/// in the I-type.
fn decode_fence_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let pred = encoded.pred;
    let succ = encoded.succ;
    let fm = encoded.fm;
    // TODO: check funct12 -- possibly parse funct12 for readability
    match encoded.funct3 {
        0b000 => {
            // rd and rs1 must be zero
            if encoded.rd != 0 || encoded.rs1 != 0 {
                return Err(Error::InvalidFormat);
            }
            if fm != 0 {
                return Err(Error::InvalidFormat);
            }
            Ok(Instruction::FENCE { pred, succ })
        }
        0b001 => {
            // rd and rs1 must be zero
            if encoded.rd != 0 || encoded.rs1 != 0 {
                return Err(Error::InvalidFormat);
            }

            // Requires `Zifencei`
            Ok(Instruction::FENCE_I)
        }
        _ => Err(Error::InvalidFormat),
    }
}

/// Decode OP-IMM instructions
///
/// Uses standard I-type format (see InstructionFormat::I)
///
/// Note: RV32I uses 5-bit shamt while RV64I uses 6-bit shamt. We will assume RV64I is enabled.
fn decode_op_imm_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let imm = encoded.i_immediate;
    // I-type doesn't use funct7, but we just re-use it to get top 7 bits
    // We could just as well shift on the immediate
    let funct7 = encoded.funct7;

    let shamt = encoded.shamt64; // We assume RV64I, so we always use shamt64

    // imm upper bits used for validation
    let imm_hi6 = ((funct7 as u32 >> 1) & MASK6) as u8; // imm[11:6]

    match encoded.funct3 {
        0b000 => Ok(Instruction::ADDI { rd, rs1, imm }),
        0b001 => {
            // SLLI: check reserved upper immediate bits
            // For RV64I, we need to check the high 6 bits.
            if imm_hi6 != 0 {
                return Err(Error::InvalidFormat);
            }

            Ok(Instruction::SLLI { rd, rs1, shamt })
        }
        0b010 => Ok(Instruction::SLTI { rd, rs1, imm }),
        0b011 => Ok(Instruction::SLTIU { rd, rs1, imm }),
        0b100 => Ok(Instruction::XORI { rd, rs1, imm }),
        0b101 => {
            // We assume RV64I, so we check the following bit sequences
            // to determine whether we need to choose SRLI or SRAI
            match imm_hi6 {
                0b000000 => Ok(Instruction::SRLI { rd, rs1, shamt }),
                0b010000 => Ok(Instruction::SRAI { rd, rs1, shamt }),
                _ => Err(Error::InvalidFormat),
            }
        }
        0b110 => Ok(Instruction::ORI { rd, rs1, imm }),
        0b111 => Ok(Instruction::ANDI { rd, rs1, imm }),
        _ => Err(Error::InvalidFormat),
    }
}

/// Decode OP-IMM-32 instructions (RV64I word immediate operations)
///
/// Uses standard I-type format (see InstructionFormat::I)
///
/// Note: All instructions in this function assume RV64I
///
/// Note: Even though these instructions are defined for RV64I,
/// for the shift related instructions, we only use a 5-bit `shamt`
/// because it is operating on a 32-bit word.
fn decode_op_imm_32_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    match encoded.funct3 {
        0b000 => {
            Ok(Instruction::ADDIW { rd: encoded.rd, rs1: encoded.rs1, imm: encoded.i_immediate })
        }
        0b001 => {
            if encoded.funct7 == 0 {
                let shamt = encoded.shamt32;
                Ok(Instruction::SLLIW { rd: encoded.rd, rs1: encoded.rs1, shamt })
            } else {
                Err(Error::InvalidFormat)
            }
        }
        0b101 => {
            let shamt = encoded.shamt32;
            match encoded.funct7 {
                0b0000000 => Ok(Instruction::SRLIW { rd: encoded.rd, rs1: encoded.rs1, shamt }),
                0b0100000 => Ok(Instruction::SRAIW { rd: encoded.rd, rs1: encoded.rs1, shamt }),
                _ => Err(Error::InvalidFormat),
            }
        }
        _ => Err(Error::InvalidFormat),
    }
}

/// Decode OP instructions (register-register operations)
///
/// Uses standard R-type format (see InstructionFormat::R)
fn decode_op_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;

    match (encoded.funct3, encoded.funct7) {
        // Base RV32I arithmetic
        (0b000, 0b000_0000) => Ok(Instruction::ADD { rd, rs1, rs2 }),
        (0b000, 0b010_0000) => Ok(Instruction::SUB { rd, rs1, rs2 }),
        (0b001, 0b000_0000) => Ok(Instruction::SLL { rd, rs1, rs2 }),
        (0b010, 0b000_0000) => Ok(Instruction::SLT { rd, rs1, rs2 }),
        (0b011, 0b000_0000) => Ok(Instruction::SLTU { rd, rs1, rs2 }),
        (0b100, 0b000_0000) => Ok(Instruction::XOR { rd, rs1, rs2 }),
        (0b101, 0b000_0000) => Ok(Instruction::SRL { rd, rs1, rs2 }),
        (0b101, 0b010_0000) => Ok(Instruction::SRA { rd, rs1, rs2 }),
        (0b110, 0b000_0000) => Ok(Instruction::OR { rd, rs1, rs2 }),
        (0b111, 0b000_0000) => Ok(Instruction::AND { rd, rs1, rs2 }),

        // Requires RV32M
        (0b000, 0b000_0001) => Ok(Instruction::MUL { rd, rs1, rs2 }),
        (0b001, 0b000_0001) => Ok(Instruction::MULH { rd, rs1, rs2 }),
        (0b010, 0b000_0001) => Ok(Instruction::MULHSU { rd, rs1, rs2 }),
        (0b011, 0b000_0001) => Ok(Instruction::MULHU { rd, rs1, rs2 }),
        (0b100, 0b000_0001) => Ok(Instruction::DIV { rd, rs1, rs2 }),
        (0b101, 0b000_0001) => Ok(Instruction::DIVU { rd, rs1, rs2 }),
        (0b110, 0b000_0001) => Ok(Instruction::REM { rd, rs1, rs2 }),
        (0b111, 0b000_0001) => Ok(Instruction::REMU { rd, rs1, rs2 }),

        _ => Err(Error::InvalidFormat),
    }
}

/// Decode OP-32 instructions (RV64I word register operations)
///
/// Uses standard R-type format (see InstructionFormat::R)  
fn decode_op_32_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;

    match (encoded.funct3, encoded.funct7) {
        // Requires RV64I
        (0b000, 0b000_0000) => Ok(Instruction::ADDW { rd, rs1, rs2 }),
        (0b000, 0b010_0000) => Ok(Instruction::SUBW { rd, rs1, rs2 }),
        (0b001, 0b000_0000) => Ok(Instruction::SLLW { rd, rs1, rs2 }),
        (0b101, 0b000_0000) => Ok(Instruction::SRLW { rd, rs1, rs2 }),
        (0b101, 0b010_0000) => Ok(Instruction::SRAW { rd, rs1, rs2 }),

        // Requires RV64M
        (0b000, 0b000_0001) => Ok(Instruction::MULW { rd, rs1, rs2 }),
        (0b100, 0b000_0001) => Ok(Instruction::DIVW { rd, rs1, rs2 }),
        (0b101, 0b000_0001) => Ok(Instruction::DIVUW { rd, rs1, rs2 }),
        (0b110, 0b000_0001) => Ok(Instruction::REMW { rd, rs1, rs2 }),
        (0b111, 0b000_0001) => Ok(Instruction::REMUW { rd, rs1, rs2 }),

        _ => Err(Error::InvalidFormat),
    }
}

/// Decode AUIPC instruction
///
/// Uses standard U-type format (see InstructionFormat::U)
fn decode_auipc_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let imm = encoded.u_immediate;
    Ok(Instruction::AUIPC { rd, imm })
}

/// Decode STORE instructions
///
/// Uses standard S-type format (see InstructionFormat::S)
fn decode_store_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;
    let offset = encoded.s_immediate;

    match encoded.funct3 {
        0b000 => Ok(Instruction::SB { rs1, rs2, offset }),
        0b001 => Ok(Instruction::SH { rs1, rs2, offset }),
        0b010 => Ok(Instruction::SW { rs1, rs2, offset }),
        0b011 => {
            // Requires RV64I
            Ok(Instruction::SD { rs1, rs2, offset })
        }
        _ => Err(Error::InvalidFormat),
    }
}

/// Decode BRANCH instructions
///
/// Uses standard B-type format (see InstructionFormat::B)
fn decode_branch_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;
    let offset = encoded.b_immediate;

    match encoded.funct3 {
        0b000 => Ok(Instruction::BEQ { rs1, rs2, offset }),
        0b001 => Ok(Instruction::BNE { rs1, rs2, offset }),
        0b100 => Ok(Instruction::BLT { rs1, rs2, offset }),
        0b101 => Ok(Instruction::BGE { rs1, rs2, offset }),
        0b110 => Ok(Instruction::BLTU { rs1, rs2, offset }),
        0b111 => Ok(Instruction::BGEU { rs1, rs2, offset }),
        _ => Err(Error::InvalidFormat),
    }
}

/// Decode JAL instruction
///
/// Uses standard J-type format (see InstructionFormat::J)
fn decode_jal_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let offset = encoded.j_immediate;
    Ok(Instruction::JAL { rd, offset })
}

/// Decode JALR instruction
///
/// Uses standard I-type format (see InstructionFormat::I)  
fn decode_jalr_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    if encoded.funct3 != 0 {
        return Err(Error::InvalidFormat);
    }
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let offset = encoded.i_immediate;
    Ok(Instruction::JALR { rd, rs1, offset })
}

/// Decode LUI instruction
///
/// Uses standard U-type format (see InstructionFormat::U)
fn decode_lui_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let imm = encoded.u_immediate;
    Ok(Instruction::LUI { rd, imm })
}

/// Decode AMO (atomic) instructions
///
/// Uses standard R-type format (see InstructionFormat::R)
fn decode_amo_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;
    let aq = encoded.aq;
    let rl = encoded.rl;

    match (encoded.funct3, encoded.funct5) {
        // Requires RV32A -- Word atomic operations
        (0b010, 0b00010) => Ok(Instruction::LR_W { rd, rs1, aq, rl }),
        (0b010, 0b00011) => Ok(Instruction::SC_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b00001) => Ok(Instruction::AMOSWAP_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b00000) => Ok(Instruction::AMOADD_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b00100) => Ok(Instruction::AMOXOR_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b01100) => Ok(Instruction::AMOAND_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b01000) => Ok(Instruction::AMOOR_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b10000) => Ok(Instruction::AMOMIN_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b10100) => Ok(Instruction::AMOMAX_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b11000) => Ok(Instruction::AMOMINU_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b11100) => Ok(Instruction::AMOMAXU_W { rd, rs1, rs2, aq, rl }),

        // Requires RV64A Doubleword atomic operations
        (0b011, 0b00010) => Ok(Instruction::LR_D { rd, rs1, aq, rl }),
        (0b011, 0b00011) => Ok(Instruction::SC_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b00001) => Ok(Instruction::AMOSWAP_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b00000) => Ok(Instruction::AMOADD_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b00100) => Ok(Instruction::AMOXOR_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b01100) => Ok(Instruction::AMOAND_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b01000) => Ok(Instruction::AMOOR_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b10000) => Ok(Instruction::AMOMIN_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b10100) => Ok(Instruction::AMOMAX_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b11000) => Ok(Instruction::AMOMINU_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b11100) => Ok(Instruction::AMOMAXU_D { rd, rs1, rs2, aq, rl }),

        _ => Err(Error::InvalidFormat),
    }
}

/// Decode SYSTEM instructions
///
/// Uses standard I-type format (see InstructionFormat::I)
fn decode_system_instruction(encoded: &EncodedInstruction) -> Result<Instruction, Error> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let csr = encoded.csr;
    let uimm = encoded.rs1; // For CSR immediate instructions, rs1 field contains immediate

    match encoded.funct3 {
        0b000 => {
            // ECALL/EBREAK distinguished by I-type immediate field
            match encoded.i_immediate {
                0 => {
                    if rd != 0 || rs1 != 0 {
                        return Err(Error::InvalidFormat);
                    }
                    Ok(Instruction::ECALL)
                }
                1 => {
                    if rd != 0 || rs1 != 0 {
                        return Err(Error::InvalidFormat);
                    }
                    Ok(Instruction::EBREAK)
                }
                _ => Err(Error::InvalidFormat),
            }
        }
        0b001 | 0b010 | 0b011 | 0b101 | 0b110 | 0b111 => {
            // Requires Zicsr
            match encoded.funct3 {
                0b001 => Ok(Instruction::CSRRW { rd, rs1, csr }),
                0b010 => Ok(Instruction::CSRRS { rd, rs1, csr }),
                0b011 => Ok(Instruction::CSRRC { rd, rs1, csr }),
                0b101 => Ok(Instruction::CSRRWI { rd, uimm, csr }),
                0b110 => Ok(Instruction::CSRRSI { rd, uimm, csr }),
                0b111 => Ok(Instruction::CSRRCI { rd, uimm, csr }),
                _ => unreachable!("`funct3` should be encoded with 3 bits"),
            }
        }
        _ => Err(Error::InvalidFormat),
    }
}
