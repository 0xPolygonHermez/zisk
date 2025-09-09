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

    let opcode = Opcode::from_bits(encoded.opcode)
        .ok_or(Error::UnsupportedInstruction { opcode_bits: encoded.opcode })?;

    match opcode {
        Opcode::Load => decode_load_instruction(&encoded),
        Opcode::MiscMem => decode_fence_instruction(&encoded),
        Opcode::OpImm => decode_op_imm_instruction(&encoded),
        Opcode::Auipc => todo!(),
        Opcode::OpImm32 => todo!(),
        Opcode::Store => todo!(),
        Opcode::Amo => todo!(),
        Opcode::Op => todo!(),
        Opcode::Lui => todo!(),
        Opcode::Op32 => todo!(),
        Opcode::Branch => todo!(),
        Opcode::Jalr => todo!(),
        Opcode::Jal => todo!(),
        Opcode::System => todo!(),
    }
}

/// Bit masks for field extraction
const MASK1: u32 = 0b1; // 1-bit mask
const MASK3: u32 = 0b111; // 3-bit mask
const MASK4: u32 = 0b1111; // 4-bit mask
const MASK5: u32 = 0b1_1111; // 5-bit mask
const MASK6: u32 = 0b11_1111; // 6-bit mask
const MASK7: u32 = 0b111_1111; // 7-bit mask
const MASK8: u32 = 0b1111_1111; // 8-bit mask
const MASK10: u32 = 0b11_1111_1111; // 10-bit mask
const MASK12: u32 = 0b1111_1111_1111; // 12-bit mask

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
