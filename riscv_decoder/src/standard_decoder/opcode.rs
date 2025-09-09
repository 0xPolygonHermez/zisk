/// RISC-V instruction format types
///
/// There are six base instruction formats that all 32-bit instructions follow.
///
/// See section `2.3 Immediate Encoding Variants` in the specs for where these are defined.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InstructionFormat {
    /// R-type: Register-register operations (add, sub, mul, etc.)
    ///
    /// Instructions of this type are encoded as follows:
    /*
    --------------------------------------------------------
    R-type | funct7 |  rs2 |  rs1 | funct3 |   rd  | opcode |
           | 31-25  |24-20 |19-15 | 14-12  | 11-7  | 6-0    |
    --------------------------------------------------------
           |aq|rl|f5|  rs2 |  rs1 | funct3 |  rd   | opcode | (AMO opcodes)
    */

    /// Observe that for atomic opcodes, `funct7` is being used to store atomic specific
    /// information, like the acquire and release bit.
    R,
    /// I-type: Immediate operations and loads (addi, lw, jalr, etc.)
    ///
    /// Instructions of this type are encoded as follows:
    /*
    --------------------------------------------------------
    I-type |   imm[11:0]    |  rs1 | funct3 |   rd  | opcode |
           |   31-20        |19-15 | 14-12  | 11-7  | 6-0    |
    --------------------------------------------------------
           |  pred|succ|fm  |  rs1 | funct3 |   rd  | opcode |  (Fence opcodes)
    */

    /// Observe that for fence opcodes, the `immediate` field is being used to
    /// store fence specific information.
    I,
    /// S-type: Store operations (sw, sb, sh, sd)
    ///
    /// Instructions of this type are encoded as follows:
    /*
    --------------------------------------------------------------
    S-type | imm[11:5] |  rs2 |  rs1 | funct3 | imm[4:0] | opcode |
           | 31-25     |24-20 |19-15 | 14-12  | 11-7     | 6-0    |
    --------------------------------------------------------------
    */
    S,
    /// B-type: Branch operations (beq, bne, blt, etc.)
    ///
    /// Instructions of this type are encoded as follows:
    /*
    ---------------------------------------------------------------------------
    B-type | imm[12] | imm[10:5] |  rs2 |  rs1 | funct3 | imm[4:1|11] | opcode |
           |   31    | 30-25     |24-20 |19-15 | 14-12  | 11-7        | 6-0    |
    ---------------------------------------------------------------------------
    */
    B,
    /// U-type: Upper immediate operations (lui, auipc)
    ///
    /// Instructions of this type are encoded as follows:
    /*
    --------------------------------------------------------------------
    U-type |                imm[31:12]                 |   rd  | opcode |
           |                31-12                      | 11-7  | 6-0    |
    --------------------------------------------------------------------
    */
    U,
    /// J-type: Jump operations (jal)
    ///
    /// Instructions of this type are encoded as follows:
    /*
    --------------------------------------------------------------------
    J-type | imm[20] | imm[10:1] | imm[11] | imm[19:12] |   rd  | opcode |
           |   31    | 30-21     |   20    | 19-12      | 11-7  | 6-0    |
    --------------------------------------------------------------------
    */
    J,
}

/// RISC-V opcodes for 32-bit instructions.
///
/// Each 32-bit instruction will reserve the bottom 7 bits for the opcode. For 32-bit instructions
/// the bit sequence of these opcodes will always have their bottom two bits set.
///
/// See: https://riscv-software-src.github.io/riscv-unified-db/manual/html/isa/isa_20240411/chapters/rv-32-64g.html#opcodemap
/// The link explicitly defines the following enum variants and their bit sequences.
///
/// Note: an opcode will generally map to multiple instructions and one must decode other fields
/// such as `funct3` and `funct7` in order to determine the exact instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::unusual_byte_groupings)]
#[repr(u8)]
pub enum Opcode {
    /// Load instructions (lb, lh, lw, ld, lbu, lhu, lwu)
    Load = 0b00_000_11,

    /// Memory ordering instructions (fence, fence.i)
    MiscMem = 0b00_011_11,

    /// Immediate arithmetic/logic operations (addi, slti, xori, etc.)
    OpImm = 0b00_100_11,

    /// Add upper immediate to PC (auipc)
    Auipc = 0b00_101_11,

    /// 32-bit immediate operations (addiw, slliw, etc.) - RV64I only
    OpImm32 = 0b00_110_11,

    /// Store instructions (sb, sh, sw, sd)
    Store = 0b01_000_11,

    /// Atomic memory operations (lr, sc, amo*) - A extension
    Amo = 0b01_011_11,

    /// Register-register operations (add, sub, mul, etc.)
    Op = 0b01_100_11,

    /// Load upper immediate (lui)
    Lui = 0b01_101_11,

    /// 32-bit register operations (addw, subw, etc.) - RV64I only
    Op32 = 0b01_110_11,

    /// Branch instructions (beq, bne, blt, etc.)
    Branch = 0b11_000_11,

    /// Jump and link register (jalr)
    Jalr = 0b11_001_11,

    /// Jump and link (jal)
    Jal = 0b11_011_11,

    /// System instructions (ecall, ebreak, csr)
    System = 0b11_100_11,
}

impl Opcode {
    #[allow(clippy::unusual_byte_groupings)]
    /// Convert from u8 to Opcode enum
    pub fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0b00_000_11 => Some(Opcode::Load),
            0b00_011_11 => Some(Opcode::MiscMem),
            0b00_100_11 => Some(Opcode::OpImm),
            0b00_101_11 => Some(Opcode::Auipc),
            0b00_110_11 => Some(Opcode::OpImm32),
            0b01_000_11 => Some(Opcode::Store),
            0b01_011_11 => Some(Opcode::Amo),
            0b01_100_11 => Some(Opcode::Op),
            0b01_101_11 => Some(Opcode::Lui),
            0b01_110_11 => Some(Opcode::Op32),
            0b11_000_11 => Some(Opcode::Branch),
            0b11_001_11 => Some(Opcode::Jalr),
            0b11_011_11 => Some(Opcode::Jal),
            0b11_100_11 => Some(Opcode::System),
            _ => None,
        }
    }
}
