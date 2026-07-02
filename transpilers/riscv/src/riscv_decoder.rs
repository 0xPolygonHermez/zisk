//! RISC-V DECODER
//!
//! Providing as a single argument a 32-bit or a 16-bit instruction, the RISC-V decoder returns
//! the instruction type and name, as well as the instruction level
//! (0, 1, 2 or 3) for 32-bit instructions.
//!
//! The instruction type is an [`RiscvInstType`] enum variant, for example: I, S, B, U, J, R, R4, C, CIW,
//! CL, CS, CA, CB or CJ.  The instruction type is used to parse the instruction operands and
//! immediate values in file riscv_interpreter.rs.  It tells the interpreter what fields are present
//! in the 32-bit (or 16-bit) instruction, their position and length.  In other words, it tells the
//! interpreter the meaning of the instruction bits.
//!
//! The instruction name is an [`RiscvInstName`] enum variant identifying the instruction mnemonic, e.g.
//! addi, lw, c.addi4spn, etc., and it is used to transpile RISC-V to Zisk assembly in file
//! riscv2zisk_context.rs.  Both enums expose `as_str()`/`Display` yielding the canonical mnemonic
//! string (used for logging and `RiscvInstruction::to_text`).
//!
//! For example: add x1, x2, x3 is encoded as a 32-bit instruction 0x003100b3, and after calling
//! RiscvDecoder::decode_32(0x003100b3) we get (RiscvInstType::R, RiscvInstName::Add, 2) as a result.  With "R" we can
//! decode the values of rd, rs1 and rs2, and with "add" we can transpile it to Zisk assembly as
//! "add x1, x2, x3".

use crate::{RiscvInstName, RiscvInstType};

/// RVD structure
pub struct RiscvDecoder {}

/// RVD implementation
impl RiscvDecoder {
    pub fn decode_32(inst: u32) -> (RiscvInstType, RiscvInstName, u64) {
        match inst & 0x7F {
            3 => {
                // Opcode 3
                match (inst >> 12) & 0x7 {
                    0 => (RiscvInstType::I, RiscvInstName::Lb, 1),
                    1 => (RiscvInstType::I, RiscvInstName::Lh, 1),
                    2 => (RiscvInstType::I, RiscvInstName::Lw, 1),
                    3 => (RiscvInstType::I, RiscvInstName::Ld, 1),
                    4 => (RiscvInstType::I, RiscvInstName::Lbu, 1),
                    5 => (RiscvInstType::I, RiscvInstName::Lhu, 1),
                    6 => (RiscvInstType::I, RiscvInstName::Lwu, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            7 => {
                // Opcode 7
                match (inst >> 12) & 0x7 {
                    0 => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                    2 => (RiscvInstType::I, RiscvInstName::Flw, 1),
                    3 => (RiscvInstType::I, RiscvInstName::Fld, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            15 => {
                // Opcode 15
                match (inst >> 12) & 0x7 {
                    0 => (RiscvInstType::F, RiscvInstName::Fence, 1),
                    1 => (RiscvInstType::F, RiscvInstName::FenceI, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            19 => {
                // Opcode 19
                match (inst >> 12) & 0x7 {
                    0 => (RiscvInstType::I, RiscvInstName::Addi, 1),
                    1 => {
                        match (inst >> 20) & 0xFFF {
                            0b011000000100 => return (RiscvInstType::I, RiscvInstName::SextB, 2),
                            0b011000000101 => return (RiscvInstType::I, RiscvInstName::SextH, 2),
                            0b011000000000 => return (RiscvInstType::I, RiscvInstName::Clz, 2),
                            0b011000000001 => return (RiscvInstType::I, RiscvInstName::Ctz, 2),
                            0b011000000010 => return (RiscvInstType::I, RiscvInstName::Cpop, 2),
                            _ => {}
                        }
                        match (inst >> 26) & 0x3F {
                            0 => (RiscvInstType::I, RiscvInstName::Slli, 2),
                            10 => (RiscvInstType::I, RiscvInstName::Bseti, 2),
                            18 => (RiscvInstType::I, RiscvInstName::Bclri, 2),
                            26 => (RiscvInstType::I, RiscvInstName::Binvi, 2),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                        }
                    }
                    2 => (RiscvInstType::I, RiscvInstName::Slti, 1),
                    3 => (RiscvInstType::I, RiscvInstName::Sltiu, 1),
                    4 => (RiscvInstType::I, RiscvInstName::Xori, 1),
                    5 => {
                        match (inst >> 20) & 0xFFF {
                            0b011010111000 => return (RiscvInstType::I, RiscvInstName::Rev8, 2),
                            0b011010000111 => return (RiscvInstType::I, RiscvInstName::Brev8, 2),
                            0b001010000111 => return (RiscvInstType::I, RiscvInstName::OrcB, 2),
                            _ => {}
                        }
                        match (inst >> 26) & 0x3F {
                            0 => (RiscvInstType::I, RiscvInstName::Srli, 2),
                            16 => (RiscvInstType::I, RiscvInstName::Srai, 2),
                            18 => (RiscvInstType::I, RiscvInstName::Bexti, 2),
                            24 => (RiscvInstType::I, RiscvInstName::Rori, 2),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                        }
                    }
                    6 => (RiscvInstType::I, RiscvInstName::Ori, 1),
                    7 => (RiscvInstType::I, RiscvInstName::Andi, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            23 => {
                // Opcode 23
                (RiscvInstType::U, RiscvInstName::Auipc, 0)
            }
            27 => {
                // Opcode 27
                match (inst >> 12) & 0x7 {
                    0 => (RiscvInstType::I, RiscvInstName::Addiw, 1),
                    1 => {
                        match (inst >> 20) & 0xFFF {
                            0b011000000000 => return (RiscvInstType::I, RiscvInstName::Clzw, 2),
                            0b011000000001 => return (RiscvInstType::I, RiscvInstName::Ctzw, 2),
                            0b011000000010 => return (RiscvInstType::I, RiscvInstName::Cpopw, 2),
                            _ => {}
                        }
                        if (inst >> 26) & 0x3F == 2 {
                            return (RiscvInstType::I, RiscvInstName::SlliUw, 2);
                        }
                        match (inst >> 25) & 0x7F {
                            0 => (RiscvInstType::I, RiscvInstName::Slliw, 2),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                        }
                    }
                    5 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::I, RiscvInstName::Srliw, 2),
                        32 => (RiscvInstType::I, RiscvInstName::Sraiw, 2),
                        48 => (RiscvInstType::I, RiscvInstName::Roriw, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            35 => {
                // Opcode 35
                match (inst >> 12) & 0x7 {
                    0 => (RiscvInstType::S, RiscvInstName::Sb, 1),
                    1 => (RiscvInstType::S, RiscvInstName::Sh, 1),
                    2 => (RiscvInstType::S, RiscvInstName::Sw, 1),
                    3 => (RiscvInstType::S, RiscvInstName::Sd, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            39 =>
            // Opcode 39
            {
                match (inst >> 12) & 0x7 {
                    2 => (RiscvInstType::S, RiscvInstName::Fsw, 1),
                    3 => (RiscvInstType::S, RiscvInstName::Fsd, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            47 => {
                // Opcode 47
                match (inst >> 12) & 0x7 {
                    2 => match (inst >> 27) & 0x1F {
                        2 => (RiscvInstType::A, RiscvInstName::LrW, 2),
                        3 => (RiscvInstType::A, RiscvInstName::ScW, 2),
                        1 => (RiscvInstType::A, RiscvInstName::AmoswapW, 2),
                        0 => (RiscvInstType::A, RiscvInstName::AmoaddW, 2),
                        4 => (RiscvInstType::A, RiscvInstName::AmoxorW, 2),
                        12 => (RiscvInstType::A, RiscvInstName::AmoandW, 2),
                        8 => (RiscvInstType::A, RiscvInstName::AmoorW, 2),
                        16 => (RiscvInstType::A, RiscvInstName::AmominW, 2),
                        20 => (RiscvInstType::A, RiscvInstName::AmomaxW, 2),
                        24 => (RiscvInstType::A, RiscvInstName::AmominuW, 2),
                        28 => (RiscvInstType::A, RiscvInstName::AmomaxuW, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    3 => match (inst >> 27) & 0x1F {
                        2 => (RiscvInstType::A, RiscvInstName::LrD, 2),
                        3 => (RiscvInstType::A, RiscvInstName::ScD, 2),
                        1 => (RiscvInstType::A, RiscvInstName::AmoswapD, 2),
                        0 => (RiscvInstType::A, RiscvInstName::AmoaddD, 2),
                        4 => (RiscvInstType::A, RiscvInstName::AmoxorD, 2),
                        12 => (RiscvInstType::A, RiscvInstName::AmoandD, 2),
                        8 => (RiscvInstType::A, RiscvInstName::AmoorD, 2),
                        16 => (RiscvInstType::A, RiscvInstName::AmominD, 2),
                        20 => (RiscvInstType::A, RiscvInstName::AmomaxD, 2),
                        24 => (RiscvInstType::A, RiscvInstName::AmominuD, 2),
                        28 => (RiscvInstType::A, RiscvInstName::AmomaxuD, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            51 => {
                // Opcode 51
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Add, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Mul, 2),
                        32 => (RiscvInstType::R, RiscvInstName::Sub, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    1 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Sll, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Mulh, 2),
                        5 => (RiscvInstType::R, RiscvInstName::Clmul, 2),
                        20 => (RiscvInstType::R, RiscvInstName::Bset, 2),
                        36 => (RiscvInstType::R, RiscvInstName::Bclr, 2),
                        48 => (RiscvInstType::R, RiscvInstName::Rol, 2),
                        52 => (RiscvInstType::R, RiscvInstName::Binv, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    2 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Slt, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Mulhsu, 2),
                        5 => (RiscvInstType::R, RiscvInstName::Clmulr, 2),
                        16 => (RiscvInstType::R, RiscvInstName::Sh1add, 2),
                        20 => (RiscvInstType::R, RiscvInstName::Xperm4, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    3 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Sltu, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Mulhu, 2),
                        5 => (RiscvInstType::R, RiscvInstName::Clmulh, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    4 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Xor, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Div, 2),
                        4 => (RiscvInstType::R, RiscvInstName::Pack, 2),
                        5 => (RiscvInstType::R, RiscvInstName::Min, 2),
                        16 => (RiscvInstType::R, RiscvInstName::Sh2add, 2),
                        20 => (RiscvInstType::R, RiscvInstName::Xperm8, 2),
                        32 => (RiscvInstType::R, RiscvInstName::Xnor, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    5 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Srl, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Divu, 2),
                        5 => (RiscvInstType::R, RiscvInstName::Minu, 2),
                        32 => (RiscvInstType::R, RiscvInstName::Sra, 2),
                        36 => (RiscvInstType::R, RiscvInstName::Bext, 2),
                        48 => (RiscvInstType::R, RiscvInstName::Ror, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    6 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Or, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Rem, 2),
                        5 => (RiscvInstType::R, RiscvInstName::Max, 2),
                        16 => (RiscvInstType::R, RiscvInstName::Sh3add, 2),
                        32 => (RiscvInstType::R, RiscvInstName::Orn, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    7 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::And, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Remu, 2),
                        4 => (RiscvInstType::R, RiscvInstName::Packh, 2),
                        5 => (RiscvInstType::R, RiscvInstName::Maxu, 2),
                        32 => (RiscvInstType::R, RiscvInstName::Andn, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            55 => {
                // Opcode 55
                (RiscvInstType::U, RiscvInstName::Lui, 0)
            }
            59 => {
                // Opcode 59
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Addw, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Mulw, 2),
                        4 => (RiscvInstType::R, RiscvInstName::AddUw, 2),
                        32 => (RiscvInstType::R, RiscvInstName::Subw, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    1 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Sllw, 2),
                        48 => (RiscvInstType::R, RiscvInstName::Rolw, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    2 => match (inst >> 25) & 0x7F {
                        16 => (RiscvInstType::R, RiscvInstName::Sh1addUw, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    4 => {
                        if (inst >> 20) & 0xFFF == 0b000010000000 {
                            return (RiscvInstType::R, RiscvInstName::ZextH, 2);
                        }
                        match (inst >> 25) & 0x7F {
                            1 => (RiscvInstType::R, RiscvInstName::Divw, 2),
                            4 => (RiscvInstType::R, RiscvInstName::Packw, 2),
                            16 => (RiscvInstType::R, RiscvInstName::Sh2addUw, 2),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                        }
                    }
                    5 => match (inst >> 25) & 0x7F {
                        0 => (RiscvInstType::R, RiscvInstName::Srlw, 2),
                        1 => (RiscvInstType::R, RiscvInstName::Divuw, 2),
                        32 => (RiscvInstType::R, RiscvInstName::Sraw, 2),
                        48 => (RiscvInstType::R, RiscvInstName::Rorw, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    6 => match (inst >> 25) & 0x7F {
                        1 => (RiscvInstType::R, RiscvInstName::Remw, 2),
                        16 => (RiscvInstType::R, RiscvInstName::Sh3addUw, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    7 => match (inst >> 25) & 0x7F {
                        1 => (RiscvInstType::R, RiscvInstName::Remuw, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            67 => {
                // Opcode 67
                match (inst >> 25) & 0x3 {
                    0 => (RiscvInstType::R4, RiscvInstName::FmaddS, 1),
                    1 => (RiscvInstType::R4, RiscvInstName::FmaddD, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            71 => {
                // Opcode 71
                match (inst >> 25) & 0x3 {
                    0 => (RiscvInstType::R4, RiscvInstName::FmsubS, 1),
                    1 => (RiscvInstType::R4, RiscvInstName::FmsubD, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            75 => {
                // Opcode 75
                match (inst >> 25) & 0x3 {
                    0 => (RiscvInstType::R4, RiscvInstName::FnmsubS, 1),
                    1 => (RiscvInstType::R4, RiscvInstName::FnmsubD, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            79 => {
                // Opcode 79
                match (inst >> 25) & 0x3 {
                    0 => (RiscvInstType::R4, RiscvInstName::FnmaddS, 1),
                    1 => (RiscvInstType::R4, RiscvInstName::FnmaddD, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            83 => {
                // Opcode 83
                match (inst >> 25) & 0x7F {
                    0 => (RiscvInstType::R, RiscvInstName::FaddS, 1),
                    1 => (RiscvInstType::R, RiscvInstName::FaddD, 1),
                    4 => (RiscvInstType::R, RiscvInstName::FsubS, 1),
                    5 => (RiscvInstType::R, RiscvInstName::FsubD, 1),
                    8 => (RiscvInstType::R, RiscvInstName::FmulS, 1),
                    9 => (RiscvInstType::R, RiscvInstName::FmulD, 1),
                    12 => (RiscvInstType::R, RiscvInstName::FdivS, 1),
                    13 => (RiscvInstType::R, RiscvInstName::FdivD, 1),
                    16 => match (inst >> 12) & 0x7 {
                        0 => (RiscvInstType::R, RiscvInstName::FsgnjS, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FsgnjnS, 2),
                        2 => (RiscvInstType::R, RiscvInstName::FsgnjxS, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    17 => match (inst >> 12) & 0x7 {
                        0 => (RiscvInstType::R, RiscvInstName::FsgnjD, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FsgnjnD, 2),
                        2 => (RiscvInstType::R, RiscvInstName::FsgnjxD, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    20 => match (inst >> 12) & 0x7 {
                        0 => (RiscvInstType::R, RiscvInstName::FminS, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FmaxS, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    21 => match (inst >> 12) & 0x7 {
                        0 => (RiscvInstType::R, RiscvInstName::FminD, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FmaxD, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    32 => match (inst >> 20) & 0x1F {
                        1 => (RiscvInstType::R, RiscvInstName::FcvtSD, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    33 => match (inst >> 20) & 0x1F {
                        0 => (RiscvInstType::R, RiscvInstName::FcvtDS, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    44 => match (inst >> 20) & 0x1F {
                        0 => (RiscvInstType::R, RiscvInstName::FsqrtS, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    45 => match (inst >> 20) & 0x1F {
                        0 => (RiscvInstType::R, RiscvInstName::FsqrtD, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    80 => match (inst >> 12) & 0x7 {
                        2 => (RiscvInstType::R, RiscvInstName::FeqS, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FltS, 2),
                        0 => (RiscvInstType::R, RiscvInstName::FleS, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    81 => match (inst >> 12) & 0x7 {
                        2 => (RiscvInstType::R, RiscvInstName::FeqD, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FltD, 2),
                        0 => (RiscvInstType::R, RiscvInstName::FleD, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    96 => match (inst >> 20) & 0x1F {
                        0 => (RiscvInstType::R, RiscvInstName::FcvtWS, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FcvtWuS, 2),
                        2 => (RiscvInstType::R, RiscvInstName::FcvtLS, 2),
                        3 => (RiscvInstType::R, RiscvInstName::FcvtLuS, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    97 => match (inst >> 20) & 0x1F {
                        0 => (RiscvInstType::R, RiscvInstName::FcvtWD, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FcvtWuD, 2),
                        2 => (RiscvInstType::R, RiscvInstName::FcvtLD, 2),
                        3 => (RiscvInstType::R, RiscvInstName::FcvtLuD, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    104 => match (inst >> 20) & 0x1F {
                        0 => (RiscvInstType::R, RiscvInstName::FcvtSW, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FcvtSWu, 2),
                        2 => (RiscvInstType::R, RiscvInstName::FcvtSL, 2),
                        3 => (RiscvInstType::R, RiscvInstName::FcvtSLu, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    105 => match (inst >> 20) & 0x1F {
                        0 => (RiscvInstType::R, RiscvInstName::FcvtDW, 2),
                        1 => (RiscvInstType::R, RiscvInstName::FcvtDWu, 2),
                        2 => (RiscvInstType::R, RiscvInstName::FcvtDL, 2),
                        3 => (RiscvInstType::R, RiscvInstName::FcvtDLu, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    112 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => (RiscvInstType::R, RiscvInstName::FmvXW, 3),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 3),
                        },
                        1 => match (inst >> 20) & 0x1F {
                            0 => (RiscvInstType::R, RiscvInstName::FclassS, 3),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 3),
                        },
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    113 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => (RiscvInstType::R, RiscvInstName::FmvXD, 3),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 3),
                        },
                        1 => match (inst >> 20) & 0x1F {
                            0 => (RiscvInstType::R, RiscvInstName::FclassD, 3),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 3),
                        },
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    120 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => (RiscvInstType::I, RiscvInstName::FmvWX, 3),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 3),
                        },
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    121 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => (RiscvInstType::I, RiscvInstName::FmvDX, 3),
                            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 3),
                        },
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            99 => {
                // Opcode 99
                match (inst >> 12) & 0x7 {
                    0 => (RiscvInstType::B, RiscvInstName::Beq, 1),
                    1 => (RiscvInstType::B, RiscvInstName::Bne, 1),
                    4 => (RiscvInstType::B, RiscvInstName::Blt, 1),
                    5 => (RiscvInstType::B, RiscvInstName::Bge, 1),
                    6 => (RiscvInstType::B, RiscvInstName::Bltu, 1),
                    7 => (RiscvInstType::B, RiscvInstName::Bgeu, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            103 => {
                // Opcode 103
                (RiscvInstType::I, RiscvInstName::Jalr, 0)
            }
            111 => {
                // Opcode 111
                (RiscvInstType::J, RiscvInstName::Jal, 0)
            }
            115 => {
                // Opcode 115
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 20) & 0xFFF {
                        0 => (RiscvInstType::C, RiscvInstName::Ecall, 2),
                        1 => (RiscvInstType::C, RiscvInstName::Ebreak, 2),
                        _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 2),
                    },
                    1 => (RiscvInstType::C, RiscvInstName::Csrrw, 1),
                    2 => (RiscvInstType::C, RiscvInstName::Csrrs, 1),
                    3 => (RiscvInstType::C, RiscvInstName::Csrrc, 1),
                    5 => (RiscvInstType::C, RiscvInstName::Csrrwi, 1),
                    6 => (RiscvInstType::C, RiscvInstName::Csrrsi, 1),
                    7 => (RiscvInstType::C, RiscvInstName::Csrrci, 1),
                    _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 1),
                }
            }
            _ => (RiscvInstType::Invalid, RiscvInstName::Reserved, 0),
        }
    }

    // Converts a compressed register index (e.g. rs1') to a full register index (e.g. rs1)
    // Source: https://www2.eecs.berkeley.edu/Pubs/TechRpts/2015/EECS-2015-209.pdf
    //     RVC Register Number 000 001 010 011 100 101 110 111
    // Integer Register Number  x8  x9 x10 x11 x12 x13 x14 x15
    pub fn convert_compressed_reg_index(reg: u32) -> u32 {
        assert!(reg < 8);
        reg + 8
    }

    // Source: https://www2.eecs.berkeley.edu/Pubs/TechRpts/2015/EECS-2015-209.pdf

    // RVC Instruction Formats:
    // Format Meaning              15 14 13 12  11 10 9 8 7 6 5 4 3 2 1 0
    // CR     Register             funct4       rd/rs1      rs2       op
    // CI     Immediate            funct3   imm rd/rs1      imm       op
    // CSS    Stack-relative Store funct3   imm             rs2       op
    // CIW    Wide Immediate       funct3   imm                 rd′   op
    // CL     Load                 funct3   imm       rs1′  imm rd′   op
    // CS     Store                funct3   imm       rs1′  imm rs2′  op
    // CA     Arithmetic           funct6             rd'/1'f2  rs2′  op
    // CB     Branch               funct3   offset    rs1′  offset    op
    // CJ     Jump                 funct3   jump target               op

    pub fn decode_16(inst: u16) -> (RiscvInstType, RiscvInstName) {
        //println!("RiscvDecoder::decode_16() inst=0x{:x}", inst);
        // Return the type and name of the instruction
        match inst & 0x3 {
            // Check bits 1 and 0 = op2
            0x00 => {
                if inst == 0x0000 {
                    return (RiscvInstType::Cinvalid, RiscvInstName::CReserved);
                }
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => (RiscvInstType::Ciw, RiscvInstName::CAddi4spn), // Mapped to addi: addi rd′, x2, nzuimm[9:2]
                    0x1 => (RiscvInstType::Cl, RiscvInstName::CFld), // Mapped to ld: ld rd′, offset(rs1′)
                    0x2 => (RiscvInstType::Cl, RiscvInstName::CLw), // Mapped to lw: lw rd′, offset(rs1′)
                    0x3 => (RiscvInstType::Cl, RiscvInstName::CLd), // Mapped to ld: ld rd′, offset(rs1′)
                    0x4 => (RiscvInstType::Cinvalid, RiscvInstName::CReserved), // Reserved
                    0x5 => (RiscvInstType::Cs, RiscvInstName::CFsd), // Mapped to sd: sd rs2′, offset(rs1′)
                    0x6 => (RiscvInstType::Cs, RiscvInstName::CSw), // Mapped to sw: sw rs2′,offset(rs1′)
                    0x7 => (RiscvInstType::Cs, RiscvInstName::CSd), // Mapped to sd: sd rs2′, offset(rs1′)
                    _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
                }
            }
            0x01 => match (inst >> 13) & 0x7 {
                // Check bits 15 to 13 = funct3
                0x0 => {
                    if ((inst >> 7) & 0x1F) == 0x0 {
                        (RiscvInstType::Ci, RiscvInstName::CNop) // Transpiled to ZisK nop (flag)
                    } else {
                        (RiscvInstType::Ci, RiscvInstName::CAddi) // Mapped to addi: addi rd, rd, imm
                    }
                }
                0x1 => (RiscvInstType::Ci, RiscvInstName::CAddiw), // Mapped to addiw: addiw rd, rd, imm
                0x2 => (RiscvInstType::Ci, RiscvInstName::CLi), // Mapped to addi: addi rd, x0, imm
                0x3 => {
                    if ((inst >> 7) & 0x1F) == 2 {
                        (RiscvInstType::Ci, RiscvInstName::CAddi16sp) // Mapped to addi: addi x2, x2, nzimm[9:4]
                    } else {
                        (RiscvInstType::Ci, RiscvInstName::CLui) // Mapped to lui: lui rd, imm
                    }
                }
                0x4 => match (inst >> 10) & 0x3 {
                    0x0 => (RiscvInstType::Cb, RiscvInstName::CSrli), // Mapped to srli: srli rd′, rd′, shamt
                    0x1 => (RiscvInstType::Cb, RiscvInstName::CSrai), // Mapped to srai: srai rd′, rd′, shamt
                    0x2 => (RiscvInstType::Cb, RiscvInstName::CAndi), // Mapped to andi: andi rd′, rd′, imm
                    0x3 => match (inst >> 12) & 0x1 {
                        0x0 => match (inst >> 5) & 0x3 {
                            0x0 => (RiscvInstType::Ca, RiscvInstName::CSub), // Mapped to sub: sub rd′, rd′, rs2′
                            0x1 => (RiscvInstType::Ca, RiscvInstName::CXor), // Mapped to xor: xor rd′, rd′, rs2′
                            0x2 => (RiscvInstType::Ca, RiscvInstName::COr), // Mapped to or: or rd′, rd′, rs2′
                            0x3 => (RiscvInstType::Ca, RiscvInstName::CAnd), // Mapped to and: and rd′, rd′, rs2′
                            _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
                        },
                        0x01 => match (inst >> 5) & 0x3 {
                            0x0 => (RiscvInstType::Ca, RiscvInstName::CSubw), // Mapped to subw: subw rd′, rd′, rs2′
                            0x1 => (RiscvInstType::Ca, RiscvInstName::CAddw), // Mapped to addw: addw rd′, rd′,rs2′
                            0x2 | 0x3 => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
                            _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
                        },
                        _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
                    },
                    _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
                },
                0x5 => (RiscvInstType::Cj, RiscvInstName::CJ), // Mapped to jal: jal x0, offset
                0x6 => (RiscvInstType::Cb, RiscvInstName::CBeqz), // Mapped to beq: beq rs1′, x0, offset
                0x7 => (RiscvInstType::Cb, RiscvInstName::CBnez), // Mapped to bne: bne rs1′, x0, offset
                _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
            },
            0x02 => {
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => (RiscvInstType::Ci, RiscvInstName::CSlli), // Mapped to slli: slli rd, rd, shamt[5:0]
                    0x1 => (RiscvInstType::Ci, RiscvInstName::CFldsp), // Mapped to ld: ld rd, offset(x2), rd!=0
                    // Would map to fld: fld rd, offset(x2), x2=sp, offset*8
                    0x2 => (RiscvInstType::Ci, RiscvInstName::CLwsp), // Mapped to lw: lw rd, offset(x2)
                    0x3 => (RiscvInstType::Ci, RiscvInstName::CLdsp), // Mapped to ld: ld rd, offset(x2), rd!=0
                    0x4 => {
                        match (inst >> 12) & 0x1 {
                            // Check bit 12
                            0x0 => {
                                match (inst >> 2) & 0x1F {
                                    // Check bits 6 to 2
                                    0x0 => (RiscvInstType::Cr, RiscvInstName::CJr), // Mapped to jalr: jalr x0, 0(rs1)
                                    _ => (RiscvInstType::Cr, RiscvInstName::CMv), // Mapped to add: add rd, x0, rs2
                                }
                            }
                            0x1 => {
                                match (inst >> 2) & 0x1F {
                                    // Check bits 6 to 2
                                    0x0 => {
                                        match (inst >> 7) & 0x1F {
                                            // Check bits 11 to 7
                                            0x0 => (RiscvInstType::Ci, RiscvInstName::CEbreak), // Mapped to ebreak
                                            _ => (RiscvInstType::Cr, RiscvInstName::CJalr), // Mapped to jalr: jalr x1, 0(rs1)
                                        }
                                    }
                                    _ => (RiscvInstType::Cr, RiscvInstName::CAdd), // Mapped to add: add rd, rd, rs2
                                }
                            }
                            _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
                        }
                    }
                    0x5 => (RiscvInstType::Css, RiscvInstName::CFsdsp), // Mapped to sd: sd rs2, offset(x2)
                    0x6 => (RiscvInstType::Css, RiscvInstName::CSwsp), // Mapped to sw: sw rs2, offset(x2)
                    0x7 => (RiscvInstType::Css, RiscvInstName::CSdsp), // Mapped to sd: sd rs2, offset(x2)
                    _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
                }
            }
            _ => (RiscvInstType::Cinvalid, RiscvInstName::CReserved),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_32() {
        let mut instruction: u32;
        let mut result: (RiscvInstType, RiscvInstName, u64);

        // ========== OPCODE 3 - Load Instructions ==========
        instruction = 0x00010003; // lb x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Lb, 1));

        instruction = 0x00011003; // lh x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Lh, 1));

        instruction = 0x00012003; // lw x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Lw, 1));

        instruction = 0x00013003; // ld x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Ld, 1));

        instruction = 0x00014003; // lbu x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Lbu, 1));

        instruction = 0x00015003; // lhu x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Lhu, 1));

        instruction = 0x00016003; // lwu x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Lwu, 1));

        // ========== OPCODE 7 - Floating-point Load ==========
        instruction = 0x00012007; // flw f0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Flw, 1));

        instruction = 0x00013007; // fld f0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Fld, 1));

        // ========== OPCODE 15 - Fence Instructions ==========
        instruction = 0x0000000f; // fence
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::F, RiscvInstName::Fence, 1));

        instruction = 0x0000100f; // fence.i
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::F, RiscvInstName::FenceI, 1));

        // ========== OPCODE 19 - Immediate Arithmetic ==========
        instruction = 0x00010013; // addi x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Addi, 1));

        instruction = 0x00011013; // slli x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Slli, 2));

        instruction = 0x60411013; // sext.b x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::SextB, 2));

        instruction = 0x60511013; // sext.h x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::SextH, 2));

        instruction = 0x60011013; // clz x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Clz, 2));

        instruction = 0x60111013; // ctz x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Ctz, 2));

        instruction = 0x60211013; // cpop x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Cpop, 2));

        instruction = 0x28011013; // bseti x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Bseti, 2));

        instruction = 0x48011013; // bclri x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Bclri, 2));

        instruction = 0x68011013; // binvi x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Binvi, 2));

        instruction = 0x00012013; // slti x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Slti, 1));

        instruction = 0x00013013; // sltiu x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Sltiu, 1));

        instruction = 0x00014013; // xori x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Xori, 1));

        instruction = 0x00015013; // srli x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Srli, 2));

        instruction = 0x40015013; // srai x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Srai, 2));

        instruction = 0x48015013; // bexti x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Bexti, 2));

        instruction = 0x60015013; // rori x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Rori, 2));

        instruction = 0x6b815013; // rev8 x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Rev8, 2));

        instruction = 0x68715013; // brev8 x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Brev8, 2));

        instruction = 0x28715013; // orc.b x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::OrcB, 2));

        instruction = 0x00016013; // ori x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Ori, 1));

        instruction = 0x00017013; // andi x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Andi, 1));

        // ========== OPCODE 23 - AUIPC ==========
        instruction = 0x00000017; // auipc x0, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::U, RiscvInstName::Auipc, 0));

        // ========== OPCODE 27 - 32-bit Immediate Arithmetic ==========
        instruction = 0x0001001b; // addiw x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Addiw, 1));

        instruction = 0x6001101b; // clzw x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Clzw, 2));

        instruction = 0x6011101b; // ctzw x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Ctzw, 2));

        instruction = 0x6021101b; // cpopw x0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Cpopw, 2));

        instruction = 0x0801101b; // slli.uw x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::SlliUw, 2));

        instruction = 0x0001101b; // slliw x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Slliw, 2));

        instruction = 0x0001501b; // srliw x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Srliw, 2));

        instruction = 0x4001501b; // sraiw x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Sraiw, 2));

        instruction = 0x6001501b; // roriw x0, x2, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Roriw, 2));

        // ========== OPCODE 35 - Store Instructions ==========
        instruction = 0x00010023; // sb x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::S, RiscvInstName::Sb, 1));

        instruction = 0x00011023; // sh x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::S, RiscvInstName::Sh, 1));

        instruction = 0x00012023; // sw x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::S, RiscvInstName::Sw, 1));

        instruction = 0x00013023; // sd x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::S, RiscvInstName::Sd, 1));

        // ========== OPCODE 39 - Floating-point Store ==========
        instruction = 0x00012027; // fsw f0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::S, RiscvInstName::Fsw, 1));

        instruction = 0x00013027; // fsd f0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::S, RiscvInstName::Fsd, 1));

        // ========== OPCODE 47 - Atomic Instructions ==========
        instruction = 0x1001202f; // lr.w x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::LrW, 2));

        instruction = 0x1801202f; // sc.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::ScW, 2));

        instruction = 0x0801202f; // amoswap.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmoswapW, 2));

        instruction = 0x0001202f; // amoadd.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmoaddW, 2));

        instruction = 0x2001202f; // amoxor.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmoxorW, 2));

        instruction = 0x6001202f; // amoand.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmoandW, 2));

        instruction = 0x4001202f; // amoor.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmoorW, 2));

        instruction = 0x8001202f; // amomin.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmominW, 2));

        instruction = 0xa001202f; // amomax.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmomaxW, 2));

        instruction = 0xc001202f; // amominu.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmominuW, 2));

        instruction = 0xe001202f; // amomaxu.w x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmomaxuW, 2));

        instruction = 0x1001302f; // lr.d x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::LrD, 2));

        instruction = 0x1801302f; // sc.d x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::ScD, 2));

        instruction = 0x0801302f; // amoswap.d x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmoswapD, 2));

        instruction = 0x0001302f; // amoadd.d x0, x0, (x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::A, RiscvInstName::AmoaddD, 2));

        // ========== OPCODE 51 - Register-Register Arithmetic ==========
        instruction = 0x003100b3; // add x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Add, 2));

        instruction = 0x023100b3; // mul x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Mul, 2));

        instruction = 0x403100b3; // sub x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sub, 2));

        instruction = 0x003110b3; // sll x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sll, 2));

        instruction = 0x023110b3; // mulh x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Mulh, 2));

        instruction = 0x0a3110b3; // clmul x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Clmul, 2));

        instruction = 0x283110b3; // bset x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Bset, 2));

        instruction = 0x483110b3; // bclr x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Bclr, 2));

        instruction = 0x603110b3; // rol x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Rol, 2));

        instruction = 0x683110b3; // binv x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Binv, 2));

        instruction = 0x003120b3; // slt x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Slt, 2));

        instruction = 0x023120b3; // mulhsu x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Mulhsu, 2));

        instruction = 0x0a3120b3; // clmulr x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Clmulr, 2));

        instruction = 0x203120b3; // sh1add x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sh1add, 2));

        instruction = 0x283120b3; // xperm4 x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Xperm4, 2));

        instruction = 0x003130b3; // sltu x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sltu, 2));

        instruction = 0x023130b3; // mulhu x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Mulhu, 2));

        instruction = 0x0a3130b3; // clmulh x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Clmulh, 2));

        instruction = 0x003140b3; // xor x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Xor, 2));

        instruction = 0x023140b3; // div x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Div, 2));

        instruction = 0x083140b3; // pack x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Pack, 2));

        instruction = 0x0a3140b3; // min x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Min, 2));

        instruction = 0x203140b3; // sh2add x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sh2add, 2));

        instruction = 0x283140b3; // xperm8 x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Xperm8, 2));

        instruction = 0x403140b3; // xnor x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Xnor, 2));

        instruction = 0x003150b3; // srl x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Srl, 2));

        instruction = 0x023150b3; // divu x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Divu, 2));

        instruction = 0x0a3150b3; // minu x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Minu, 2));

        instruction = 0x403150b3; // sra x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sra, 2));

        instruction = 0x483150b3; // bext x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Bext, 2));

        instruction = 0x603150b3; // ror x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Ror, 2));

        instruction = 0x003160b3; // or x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Or, 2));

        instruction = 0x023160b3; // rem x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Rem, 2));

        instruction = 0x0a3160b3; // max x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Max, 2));

        instruction = 0x203160b3; // sh3add x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sh3add, 2));

        instruction = 0x403160b3; // orn x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Orn, 2));

        instruction = 0x003170b3; // and x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::And, 2));

        instruction = 0x023170b3; // remu x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Remu, 2));

        instruction = 0x083170b3; // packh x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Packh, 2));

        instruction = 0x0a3170b3; // maxu x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Maxu, 2));

        instruction = 0x403170b3; // andn x1, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Andn, 2));

        // ========== OPCODE 55 - LUI ==========
        instruction = 0x00000037; // lui x0, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::U, RiscvInstName::Lui, 0));

        // ========== OPCODE 59 - 32-bit Register-Register Arithmetic ==========
        instruction = 0x0031003b; // addw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Addw, 2));

        instruction = 0x0231003b; // mulw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Mulw, 2));

        instruction = 0x0831003b; // add.uw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::AddUw, 2));

        instruction = 0x4031003b; // subw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Subw, 2));

        instruction = 0x0031103b; // sllw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sllw, 2));

        instruction = 0x6031103b; // rolw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Rolw, 2));

        instruction = 0x2031203b; // sh1add.uw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sh1addUw, 2));

        instruction = 0x0800403b; // zext.h x0, x0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::ZextH, 2));

        instruction = 0x0231403b; // divw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Divw, 2));

        instruction = 0x0831403b; // packw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Packw, 2));

        instruction = 0x2031403b; // sh2add.uw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sh2addUw, 2));

        instruction = 0x0031503b; // srlw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Srlw, 2));

        instruction = 0x0231503b; // divuw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Divuw, 2));

        instruction = 0x4031503b; // sraw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sraw, 2));

        instruction = 0x6031503b; // rorw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Rorw, 2));

        instruction = 0x0231603b; // remw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Remw, 2));

        instruction = 0x2031603b; // sh3add.uw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Sh3addUw, 2));

        instruction = 0x0231703b; // remuw x0, x2, x3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::Remuw, 2));

        // ========== OPCODE 67, 71, 75, 79 - Floating-point Fused Multiply-Add ==========
        instruction = 0x00310043; // fmadd.s f0, f2, f3, f0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R4, RiscvInstName::FmaddS, 1));

        instruction = 0x02310043; // fmadd.d f0, f2, f3, f0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R4, RiscvInstName::FmaddD, 1));

        instruction = 0x00310047; // fmsub.s f0, f2, f3, f0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R4, RiscvInstName::FmsubS, 1));

        instruction = 0x0231004b; // fnmsub.d f0, f2, f3, f0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R4, RiscvInstName::FnmsubD, 1));

        instruction = 0x0031004f; // fnmadd.s f0, f2, f3, f0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R4, RiscvInstName::FnmaddS, 1));

        // ========== OPCODE 83 - Floating-point Arithmetic ==========
        instruction = 0x00310053; // fadd.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FaddS, 1));

        instruction = 0x02310053; // fadd.d f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FaddD, 1));

        instruction = 0x08310053; // fsub.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FsubS, 1));

        instruction = 0x0a310053; // fsub.d f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FsubD, 1));

        instruction = 0x10310053; // fmul.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FmulS, 1));

        instruction = 0x12310053; // fmul.d f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FmulD, 1));

        instruction = 0x18310053; // fdiv.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FdivS, 1));

        instruction = 0x1a310053; // fdiv.d f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FdivD, 1));

        instruction = 0x20310053; // fsgnj.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FsgnjS, 2));

        instruction = 0x20311053; // fsgnjn.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FsgnjnS, 2));

        instruction = 0x20312053; // fsgnjx.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FsgnjxS, 2));

        instruction = 0x22310053; // fsgnj.d f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FsgnjD, 2));

        instruction = 0x28310053; // fmin.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FminS, 2));

        instruction = 0x28311053; // fmax.s f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FmaxS, 2));

        instruction = 0x2a310053; // fmin.d f0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FminD, 2));

        instruction = 0x40110053; // fcvt.s.d f0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtSD, 2));

        instruction = 0x42010053; // fcvt.d.s f0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtDS, 2));

        instruction = 0x58010053; // fsqrt.s f0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FsqrtS, 2));

        instruction = 0x5a010053; // fsqrt.d f0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FsqrtD, 2));

        instruction = 0xa0312053; // feq.s x0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FeqS, 2));

        instruction = 0xa0311053; // flt.s x0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FltS, 2));

        instruction = 0xa0310053; // fle.s x0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FleS, 2));

        instruction = 0xa2312053; // feq.d x0, f2, f3
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FeqD, 2));

        instruction = 0xc0010053; // fcvt.w.s x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtWS, 2));

        instruction = 0xc0110053; // fcvt.wu.s x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtWuS, 2));

        instruction = 0xc0210053; // fcvt.l.s x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtLS, 2));

        instruction = 0xc0310053; // fcvt.lu.s x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtLuS, 2));

        instruction = 0xc2010053; // fcvt.w.d x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtWD, 2));

        instruction = 0xc2210053; // fcvt.l.d x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtLD, 2));

        instruction = 0xd0010053; // fcvt.s.w f0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtSW, 2));

        instruction = 0xd0110053; // fcvt.s.wu f0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtSWu, 2));

        instruction = 0xd0210053; // fcvt.s.l f0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtSL, 2));

        instruction = 0xd2010053; // fcvt.d.w f0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtDW, 2));

        instruction = 0xd2210053; // fcvt.d.l f0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FcvtDL, 2));

        instruction = 0xe0010053; // fmv.x.w x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FmvXW, 3));

        instruction = 0xe0011053; // fclass.s x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FclassS, 3));

        instruction = 0xe2010053; // fmv.x.d x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FmvXD, 3));

        instruction = 0xe2011053; // fclass.d x0, f2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::R, RiscvInstName::FclassD, 3));

        instruction = 0xf0010053; // fmv.w.x f0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::FmvWX, 3));

        instruction = 0xf2010053; // fmv.d.x f0, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::FmvDX, 3));

        // ========== OPCODE 99 - Branch Instructions ==========
        instruction = 0x00310063; // beq x2, x3, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::B, RiscvInstName::Beq, 1));

        instruction = 0x00311063; // bne x2, x3, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::B, RiscvInstName::Bne, 1));

        instruction = 0x00314063; // blt x2, x3, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::B, RiscvInstName::Blt, 1));

        instruction = 0x00315063; // bge x2, x3, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::B, RiscvInstName::Bge, 1));

        instruction = 0x00316063; // bltu x2, x3, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::B, RiscvInstName::Bltu, 1));

        instruction = 0x00317063; // bgeu x2, x3, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::B, RiscvInstName::Bgeu, 1));

        // ========== OPCODE 103 - JALR ==========
        instruction = 0x00010067; // jalr x0, 0(x2)
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::I, RiscvInstName::Jalr, 0));

        // ========== OPCODE 111 - JAL ==========
        instruction = 0x0000006f; // jal x0, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::J, RiscvInstName::Jal, 0));

        // ========== OPCODE 115 - System Instructions ==========
        instruction = 0x00000073; // ecall
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::C, RiscvInstName::Ecall, 2));

        instruction = 0x00100073; // ebreak
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::C, RiscvInstName::Ebreak, 2));

        instruction = 0x00011073; // csrrw x0, 0x001, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::C, RiscvInstName::Csrrw, 1));

        instruction = 0x00012073; // csrrs x0, 0x001, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::C, RiscvInstName::Csrrs, 1));

        instruction = 0x00013073; // csrrc x0, 0x001, x2
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::C, RiscvInstName::Csrrc, 1));

        instruction = 0x00015073; // csrrwi x0, 0x001, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::C, RiscvInstName::Csrrwi, 1));

        instruction = 0x00016073; // csrrsi x0, 0x001, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::C, RiscvInstName::Csrrsi, 1));

        instruction = 0x00017073; // csrrci x0, 0x001, 0
        result = RiscvDecoder::decode_32(instruction);
        assert_eq!(result, (RiscvInstType::C, RiscvInstName::Csrrci, 1));
    }

    #[test]
    fn test_decode_16() {
        let mut instruction: u16;
        let mut result: (RiscvInstType, RiscvInstName);

        // ========== OP2 = 0x00 ==========
        instruction = 0x0000; // reserved all-zero encoding
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cinvalid, RiscvInstName::CReserved));

        instruction = 0x0004; // c.addi4spn
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ciw, RiscvInstName::CAddi4spn));

        instruction = 0x2000; // c.fld
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cl, RiscvInstName::CFld));

        instruction = 0x4000; // c.lw
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cl, RiscvInstName::CLw));

        instruction = 0x6000; // c.ld
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cl, RiscvInstName::CLd));

        instruction = 0x8000; // reserved
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cinvalid, RiscvInstName::CReserved));

        instruction = 0xa000; // c.fsd
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cs, RiscvInstName::CFsd));

        instruction = 0xc000; // c.sw
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cs, RiscvInstName::CSw));

        instruction = 0xe000; // c.sd
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cs, RiscvInstName::CSd));

        // ========== OP2 = 0x01 ==========
        instruction = 0x0001; // c.nop
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CNop));

        instruction = 0x0081; // c.addi
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CAddi));

        instruction = 0x2001; // c.addiw
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CAddiw));

        instruction = 0x4001; // c.li
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CLi));

        instruction = 0x6101; // c.addi16sp
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CAddi16sp));

        instruction = 0x6181; // c.lui
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CLui));

        instruction = 0x8001; // c.srli
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cb, RiscvInstName::CSrli));

        instruction = 0x8401; // c.srai
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cb, RiscvInstName::CSrai));

        instruction = 0x8801; // c.andi
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cb, RiscvInstName::CAndi));

        instruction = 0x8c01; // c.sub
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ca, RiscvInstName::CSub));

        instruction = 0x8c21; // c.xor
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ca, RiscvInstName::CXor));

        instruction = 0x8c41; // c.or
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ca, RiscvInstName::COr));

        instruction = 0x8c61; // c.and
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ca, RiscvInstName::CAnd));

        instruction = 0x9c01; // c.subw
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ca, RiscvInstName::CSubw));

        instruction = 0x9c21; // c.addw
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ca, RiscvInstName::CAddw));

        instruction = 0x9c41; // reserved
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cinvalid, RiscvInstName::CReserved));

        instruction = 0xa001; // c.j
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cj, RiscvInstName::CJ));

        instruction = 0xc001; // c.beqz
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cb, RiscvInstName::CBeqz));

        instruction = 0xe001; // c.bnez
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cb, RiscvInstName::CBnez));

        // ========== OP2 = 0x02 ==========
        instruction = 0x0002; // c.slli
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CSlli));

        instruction = 0x2002; // c.fldsp
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CFldsp));

        instruction = 0x4002; // c.lwsp
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CLwsp));

        instruction = 0x6002; // c.ldsp
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CLdsp));

        instruction = 0x8002; // c.jr
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cr, RiscvInstName::CJr));

        instruction = 0x8006; // c.mv
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cr, RiscvInstName::CMv));

        instruction = 0x9002; // c.ebreak
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Ci, RiscvInstName::CEbreak));

        instruction = 0x9082; // c.jalr
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cr, RiscvInstName::CJalr));

        instruction = 0x9006; // c.add
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cr, RiscvInstName::CAdd));

        instruction = 0xa002; // c.fsdsp
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Css, RiscvInstName::CFsdsp));

        instruction = 0xc002; // c.swsp
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Css, RiscvInstName::CSwsp));

        instruction = 0xe002; // c.sdsp
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Css, RiscvInstName::CSdsp));

        // ========== Unknown OP2 ==========
        instruction = 0x0003;
        result = RiscvDecoder::decode_16(instruction);
        assert_eq!(result, (RiscvInstType::Cinvalid, RiscvInstName::CReserved));
    }
}
