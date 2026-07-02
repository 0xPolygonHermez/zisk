//! RISC-V DECODER (RVD)
//!
//! Providing as a single argument a 32-bit or a 16-bit instruction, the RISC-V decoder returns
//! the instruction type and name, as well as the instruction level
//! (0, 1, 2 or 3) for 32-bit instructions.
//!
//! The instruction type is an [`InstType`] enum variant, for example: I, S, B, U, J, R, R4, C, CIW,
//! CL, CS, CA, CB or CJ.  The instruction type is used to parse the instruction operands and
//! immediate values in file riscv_interpreter.rs.  It tells the interpreter what fields are present
//! in the 32-bit (or 16-bit) instruction, their position and length.  In other words, it tells the
//! interpreter the meaning of the instruction bits.
//!
//! The instruction name is an [`InstName`] enum variant identifying the instruction mnemonic, e.g.
//! addi, lw, c.addi4spn, etc., and it is used to transpile RISC-V to Zisk assembly in file
//! riscv2zisk_context.rs.  Both enums expose `as_str()`/`Display` yielding the canonical mnemonic
//! string (used for logging and `RiscvInstruction::to_text`).
//!
//! For example: add x1, x2, x3 is encoded as a 32-bit instruction 0x003100b3, and after calling
//! RiscvDecoder::get_type_and_name_32_bits(0x003100b3) we get (InstType::R, InstName::Add, 2) as a result.  With "R" we can
//! decode the values of rd, rs1 and rs2, and with "add" we can transpile it to Zisk assembly as
//! "add x1, x2, x3".

use crate::{InstName, InstType};

/// RVD structure
pub struct RiscvDecoder {}

/// RVD implementation
impl RiscvDecoder {
    pub fn get_type_and_name_32_bits(inst: u32) -> (InstType, InstName, u64) {
        match inst & 0x7F {
            3 => {
                // Opcode 3
                match (inst >> 12) & 0x7 {
                    0 => (InstType::I, InstName::Lb, 1),
                    1 => (InstType::I, InstName::Lh, 1),
                    2 => (InstType::I, InstName::Lw, 1),
                    3 => (InstType::I, InstName::Ld, 1),
                    4 => (InstType::I, InstName::Lbu, 1),
                    5 => (InstType::I, InstName::Lhu, 1),
                    6 => (InstType::I, InstName::Lwu, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            7 => {
                // Opcode 7
                match (inst >> 12) & 0x7 {
                    0 => (InstType::Invalid, InstName::Reserved, 1),
                    2 => (InstType::I, InstName::Flw, 1),
                    3 => (InstType::I, InstName::Fld, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            15 => {
                // Opcode 15
                match (inst >> 12) & 0x7 {
                    0 => (InstType::F, InstName::Fence, 1),
                    1 => (InstType::F, InstName::FenceI, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            19 => {
                // Opcode 19
                match (inst >> 12) & 0x7 {
                    0 => (InstType::I, InstName::Addi, 1),
                    1 => {
                        match (inst >> 20) & 0xFFF {
                            0b011000000100 => return (InstType::I, InstName::SextB, 2),
                            0b011000000101 => return (InstType::I, InstName::SextH, 2),
                            0b011000000000 => return (InstType::I, InstName::Clz, 2),
                            0b011000000001 => return (InstType::I, InstName::Ctz, 2),
                            0b011000000010 => return (InstType::I, InstName::Cpop, 2),
                            _ => {}
                        }
                        match (inst >> 26) & 0x3F {
                            0 => (InstType::I, InstName::Slli, 2),
                            10 => (InstType::I, InstName::Bseti, 2),
                            18 => (InstType::I, InstName::Bclri, 2),
                            26 => (InstType::I, InstName::Binvi, 2),
                            _ => (InstType::Invalid, InstName::Reserved, 2),
                        }
                    }
                    2 => (InstType::I, InstName::Slti, 1),
                    3 => (InstType::I, InstName::Sltiu, 1),
                    4 => (InstType::I, InstName::Xori, 1),
                    5 => {
                        match (inst >> 20) & 0xFFF {
                            0b011010111000 => return (InstType::I, InstName::Rev8, 2),
                            0b011010000111 => return (InstType::I, InstName::Brev8, 2),
                            0b001010000111 => return (InstType::I, InstName::OrcB, 2),
                            _ => {}
                        }
                        match (inst >> 26) & 0x3F {
                            0 => (InstType::I, InstName::Srli, 2),
                            16 => (InstType::I, InstName::Srai, 2),
                            18 => (InstType::I, InstName::Bexti, 2),
                            24 => (InstType::I, InstName::Rori, 2),
                            _ => (InstType::Invalid, InstName::Reserved, 2),
                        }
                    }
                    6 => (InstType::I, InstName::Ori, 1),
                    7 => (InstType::I, InstName::Andi, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            23 => {
                // Opcode 23
                (InstType::U, InstName::Auipc, 0)
            }
            27 => {
                // Opcode 27
                match (inst >> 12) & 0x7 {
                    0 => (InstType::I, InstName::Addiw, 1),
                    1 => {
                        match (inst >> 20) & 0xFFF {
                            0b011000000000 => return (InstType::I, InstName::Clzw, 2),
                            0b011000000001 => return (InstType::I, InstName::Ctzw, 2),
                            0b011000000010 => return (InstType::I, InstName::Cpopw, 2),
                            _ => {}
                        }
                        if (inst >> 26) & 0x3F == 2 {
                            return (InstType::I, InstName::SlliUw, 2);
                        }
                        match (inst >> 25) & 0x7F {
                            0 => (InstType::I, InstName::Slliw, 2),
                            _ => (InstType::Invalid, InstName::Reserved, 2),
                        }
                    }
                    5 => match (inst >> 25) & 0x7F {
                        0 => (InstType::I, InstName::Srliw, 2),
                        32 => (InstType::I, InstName::Sraiw, 2),
                        48 => (InstType::I, InstName::Roriw, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            35 => {
                // Opcode 35
                match (inst >> 12) & 0x7 {
                    0 => (InstType::S, InstName::Sb, 1),
                    1 => (InstType::S, InstName::Sh, 1),
                    2 => (InstType::S, InstName::Sw, 1),
                    3 => (InstType::S, InstName::Sd, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            39 =>
            // Opcode 39
            {
                match (inst >> 12) & 0x7 {
                    2 => (InstType::S, InstName::Fsw, 1),
                    3 => (InstType::S, InstName::Fsd, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            47 => {
                // Opcode 47
                match (inst >> 12) & 0x7 {
                    2 => match (inst >> 27) & 0x1F {
                        2 => (InstType::A, InstName::LrW, 2),
                        3 => (InstType::A, InstName::ScW, 2),
                        1 => (InstType::A, InstName::AmoswapW, 2),
                        0 => (InstType::A, InstName::AmoaddW, 2),
                        4 => (InstType::A, InstName::AmoxorW, 2),
                        12 => (InstType::A, InstName::AmoandW, 2),
                        8 => (InstType::A, InstName::AmoorW, 2),
                        16 => (InstType::A, InstName::AmominW, 2),
                        20 => (InstType::A, InstName::AmomaxW, 2),
                        24 => (InstType::A, InstName::AmominuW, 2),
                        28 => (InstType::A, InstName::AmomaxuW, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    3 => match (inst >> 27) & 0x1F {
                        2 => (InstType::A, InstName::LrD, 2),
                        3 => (InstType::A, InstName::ScD, 2),
                        1 => (InstType::A, InstName::AmoswapD, 2),
                        0 => (InstType::A, InstName::AmoaddD, 2),
                        4 => (InstType::A, InstName::AmoxorD, 2),
                        12 => (InstType::A, InstName::AmoandD, 2),
                        8 => (InstType::A, InstName::AmoorD, 2),
                        16 => (InstType::A, InstName::AmominD, 2),
                        20 => (InstType::A, InstName::AmomaxD, 2),
                        24 => (InstType::A, InstName::AmominuD, 2),
                        28 => (InstType::A, InstName::AmomaxuD, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            51 => {
                // Opcode 51
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Add, 2),
                        1 => (InstType::R, InstName::Mul, 2),
                        32 => (InstType::R, InstName::Sub, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    1 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Sll, 2),
                        1 => (InstType::R, InstName::Mulh, 2),
                        5 => (InstType::R, InstName::Clmul, 2),
                        20 => (InstType::R, InstName::Bset, 2),
                        36 => (InstType::R, InstName::Bclr, 2),
                        48 => (InstType::R, InstName::Rol, 2),
                        52 => (InstType::R, InstName::Binv, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    2 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Slt, 2),
                        1 => (InstType::R, InstName::Mulhsu, 2),
                        5 => (InstType::R, InstName::Clmulr, 2),
                        16 => (InstType::R, InstName::Sh1add, 2),
                        20 => (InstType::R, InstName::Xperm4, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    3 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Sltu, 2),
                        1 => (InstType::R, InstName::Mulhu, 2),
                        5 => (InstType::R, InstName::Clmulh, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    4 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Xor, 2),
                        1 => (InstType::R, InstName::Div, 2),
                        4 => (InstType::R, InstName::Pack, 2),
                        5 => (InstType::R, InstName::Min, 2),
                        16 => (InstType::R, InstName::Sh2add, 2),
                        20 => (InstType::R, InstName::Xperm8, 2),
                        32 => (InstType::R, InstName::Xnor, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    5 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Srl, 2),
                        1 => (InstType::R, InstName::Divu, 2),
                        5 => (InstType::R, InstName::Minu, 2),
                        32 => (InstType::R, InstName::Sra, 2),
                        36 => (InstType::R, InstName::Bext, 2),
                        48 => (InstType::R, InstName::Ror, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    6 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Or, 2),
                        1 => (InstType::R, InstName::Rem, 2),
                        5 => (InstType::R, InstName::Max, 2),
                        16 => (InstType::R, InstName::Sh3add, 2),
                        32 => (InstType::R, InstName::Orn, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    7 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::And, 2),
                        1 => (InstType::R, InstName::Remu, 2),
                        4 => (InstType::R, InstName::Packh, 2),
                        5 => (InstType::R, InstName::Maxu, 2),
                        32 => (InstType::R, InstName::Andn, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            55 => {
                // Opcode 55
                (InstType::U, InstName::Lui, 0)
            }
            59 => {
                // Opcode 59
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Addw, 2),
                        1 => (InstType::R, InstName::Mulw, 2),
                        4 => (InstType::R, InstName::AddUw, 2),
                        32 => (InstType::R, InstName::Subw, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    1 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Sllw, 2),
                        48 => (InstType::R, InstName::Rolw, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    2 => match (inst >> 25) & 0x7F {
                        16 => (InstType::R, InstName::Sh1addUw, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    4 => {
                        if (inst >> 20) & 0xFFF == 0b000010000000 {
                            return (InstType::R, InstName::ZextH, 2);
                        }
                        match (inst >> 25) & 0x7F {
                            1 => (InstType::R, InstName::Divw, 2),
                            4 => (InstType::R, InstName::Packw, 2),
                            16 => (InstType::R, InstName::Sh2addUw, 2),
                            _ => (InstType::Invalid, InstName::Reserved, 2),
                        }
                    }
                    5 => match (inst >> 25) & 0x7F {
                        0 => (InstType::R, InstName::Srlw, 2),
                        1 => (InstType::R, InstName::Divuw, 2),
                        32 => (InstType::R, InstName::Sraw, 2),
                        48 => (InstType::R, InstName::Rorw, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    6 => match (inst >> 25) & 0x7F {
                        1 => (InstType::R, InstName::Remw, 2),
                        16 => (InstType::R, InstName::Sh3addUw, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    7 => match (inst >> 25) & 0x7F {
                        1 => (InstType::R, InstName::Remuw, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            67 => {
                // Opcode 67
                match (inst >> 25) & 0x3 {
                    0 => (InstType::R4, InstName::FmaddS, 1),
                    1 => (InstType::R4, InstName::FmaddD, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            71 => {
                // Opcode 71
                match (inst >> 25) & 0x3 {
                    0 => (InstType::R4, InstName::FmsubS, 1),
                    1 => (InstType::R4, InstName::FmsubD, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            75 => {
                // Opcode 75
                match (inst >> 25) & 0x3 {
                    0 => (InstType::R4, InstName::FnmsubS, 1),
                    1 => (InstType::R4, InstName::FnmsubD, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            79 => {
                // Opcode 79
                match (inst >> 25) & 0x3 {
                    0 => (InstType::R4, InstName::FnmaddS, 1),
                    1 => (InstType::R4, InstName::FnmaddD, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            83 => {
                // Opcode 83
                match (inst >> 25) & 0x7F {
                    0 => (InstType::R, InstName::FaddS, 1),
                    1 => (InstType::R, InstName::FaddD, 1),
                    4 => (InstType::R, InstName::FsubS, 1),
                    5 => (InstType::R, InstName::FsubD, 1),
                    8 => (InstType::R, InstName::FmulS, 1),
                    9 => (InstType::R, InstName::FmulD, 1),
                    12 => (InstType::R, InstName::FdivS, 1),
                    13 => (InstType::R, InstName::FdivD, 1),
                    16 => match (inst >> 12) & 0x7 {
                        0 => (InstType::R, InstName::FsgnjS, 2),
                        1 => (InstType::R, InstName::FsgnjnS, 2),
                        2 => (InstType::R, InstName::FsgnjxS, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    17 => match (inst >> 12) & 0x7 {
                        0 => (InstType::R, InstName::FsgnjD, 2),
                        1 => (InstType::R, InstName::FsgnjnD, 2),
                        2 => (InstType::R, InstName::FsgnjxD, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    20 => match (inst >> 12) & 0x7 {
                        0 => (InstType::R, InstName::FminS, 2),
                        1 => (InstType::R, InstName::FmaxS, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    21 => match (inst >> 12) & 0x7 {
                        0 => (InstType::R, InstName::FminD, 2),
                        1 => (InstType::R, InstName::FmaxD, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    32 => match (inst >> 20) & 0x1F {
                        1 => (InstType::R, InstName::FcvtSD, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    33 => match (inst >> 20) & 0x1F {
                        0 => (InstType::R, InstName::FcvtDS, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    44 => match (inst >> 20) & 0x1F {
                        0 => (InstType::R, InstName::FsqrtS, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    45 => match (inst >> 20) & 0x1F {
                        0 => (InstType::R, InstName::FsqrtD, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    80 => match (inst >> 12) & 0x7 {
                        2 => (InstType::R, InstName::FeqS, 2),
                        1 => (InstType::R, InstName::FltS, 2),
                        0 => (InstType::R, InstName::FleS, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    81 => match (inst >> 12) & 0x7 {
                        2 => (InstType::R, InstName::FeqD, 2),
                        1 => (InstType::R, InstName::FltD, 2),
                        0 => (InstType::R, InstName::FleD, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    96 => match (inst >> 20) & 0x1F {
                        0 => (InstType::R, InstName::FcvtWS, 2),
                        1 => (InstType::R, InstName::FcvtWuS, 2),
                        2 => (InstType::R, InstName::FcvtLS, 2),
                        3 => (InstType::R, InstName::FcvtLuS, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    97 => match (inst >> 20) & 0x1F {
                        0 => (InstType::R, InstName::FcvtWD, 2),
                        1 => (InstType::R, InstName::FcvtWuD, 2),
                        2 => (InstType::R, InstName::FcvtLD, 2),
                        3 => (InstType::R, InstName::FcvtLuD, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    104 => match (inst >> 20) & 0x1F {
                        0 => (InstType::R, InstName::FcvtSW, 2),
                        1 => (InstType::R, InstName::FcvtSWu, 2),
                        2 => (InstType::R, InstName::FcvtSL, 2),
                        3 => (InstType::R, InstName::FcvtSLu, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    105 => match (inst >> 20) & 0x1F {
                        0 => (InstType::R, InstName::FcvtDW, 2),
                        1 => (InstType::R, InstName::FcvtDWu, 2),
                        2 => (InstType::R, InstName::FcvtDL, 2),
                        3 => (InstType::R, InstName::FcvtDLu, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    112 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => (InstType::R, InstName::FmvXW, 3),
                            _ => (InstType::Invalid, InstName::Reserved, 3),
                        },
                        1 => match (inst >> 20) & 0x1F {
                            0 => (InstType::R, InstName::FclassS, 3),
                            _ => (InstType::Invalid, InstName::Reserved, 3),
                        },
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    113 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => (InstType::R, InstName::FmvXD, 3),
                            _ => (InstType::Invalid, InstName::Reserved, 3),
                        },
                        1 => match (inst >> 20) & 0x1F {
                            0 => (InstType::R, InstName::FclassD, 3),
                            _ => (InstType::Invalid, InstName::Reserved, 3),
                        },
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    120 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => (InstType::I, InstName::FmvWX, 3),
                            _ => (InstType::Invalid, InstName::Reserved, 3),
                        },
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    121 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => (InstType::I, InstName::FmvDX, 3),
                            _ => (InstType::Invalid, InstName::Reserved, 3),
                        },
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            99 => {
                // Opcode 99
                match (inst >> 12) & 0x7 {
                    0 => (InstType::B, InstName::Beq, 1),
                    1 => (InstType::B, InstName::Bne, 1),
                    4 => (InstType::B, InstName::Blt, 1),
                    5 => (InstType::B, InstName::Bge, 1),
                    6 => (InstType::B, InstName::Bltu, 1),
                    7 => (InstType::B, InstName::Bgeu, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            103 => {
                // Opcode 103
                (InstType::I, InstName::Jalr, 0)
            }
            111 => {
                // Opcode 111
                (InstType::J, InstName::Jal, 0)
            }
            115 => {
                // Opcode 115
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 20) & 0xFFF {
                        0 => (InstType::C, InstName::Ecall, 2),
                        1 => (InstType::C, InstName::Ebreak, 2),
                        _ => (InstType::Invalid, InstName::Reserved, 2),
                    },
                    1 => (InstType::C, InstName::Csrrw, 1),
                    2 => (InstType::C, InstName::Csrrs, 1),
                    3 => (InstType::C, InstName::Csrrc, 1),
                    5 => (InstType::C, InstName::Csrrwi, 1),
                    6 => (InstType::C, InstName::Csrrsi, 1),
                    7 => (InstType::C, InstName::Csrrci, 1),
                    _ => (InstType::Invalid, InstName::Reserved, 1),
                }
            }
            _ => (InstType::Invalid, InstName::Reserved, 0),
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

    pub fn get_type_and_name_16_bits(inst: u16) -> (InstType, InstName) {
        //println!("RiscvDecoder::get_type_and_name_16_bits() inst=0x{:x}", inst);
        // Return the type and name of the instruction
        match inst & 0x3 {
            // Check bits 1 and 0 = op2
            0x00 => {
                if inst == 0x0000 {
                    return (InstType::Cinvalid, InstName::CReserved);
                }
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => (InstType::Ciw, InstName::CAddi4spn), // Mapped to addi: addi rd′, x2, nzuimm[9:2]
                    0x1 => (InstType::Cl, InstName::CFld), // Mapped to ld: ld rd′, offset(rs1′)
                    0x2 => (InstType::Cl, InstName::CLw),  // Mapped to lw: lw rd′, offset(rs1′)
                    0x3 => (InstType::Cl, InstName::CLd),  // Mapped to ld: ld rd′, offset(rs1′)
                    0x4 => (InstType::Cinvalid, InstName::CReserved), // Reserved
                    0x5 => (InstType::Cs, InstName::CFsd), // Mapped to sd: sd rs2′, offset(rs1′)
                    0x6 => (InstType::Cs, InstName::CSw),  // Mapped to sw: sw rs2′,offset(rs1′)
                    0x7 => (InstType::Cs, InstName::CSd),  // Mapped to sd: sd rs2′, offset(rs1′)
                    _ => (InstType::Cinvalid, InstName::CReserved),
                }
            }
            0x01 => match (inst >> 13) & 0x7 {
                // Check bits 15 to 13 = funct3
                0x0 => {
                    if ((inst >> 7) & 0x1F) == 0x0 {
                        (InstType::Ci, InstName::CNop) // Transpiled to ZisK nop (flag)
                    } else {
                        (InstType::Ci, InstName::CAddi) // Mapped to addi: addi rd, rd, imm
                    }
                }
                0x1 => (InstType::Ci, InstName::CAddiw), // Mapped to addiw: addiw rd, rd, imm
                0x2 => (InstType::Ci, InstName::CLi),    // Mapped to addi: addi rd, x0, imm
                0x3 => {
                    if ((inst >> 7) & 0x1F) == 2 {
                        (InstType::Ci, InstName::CAddi16sp) // Mapped to addi: addi x2, x2, nzimm[9:4]
                    } else {
                        (InstType::Ci, InstName::CLui) // Mapped to lui: lui rd, imm
                    }
                }
                0x4 => match (inst >> 10) & 0x3 {
                    0x0 => (InstType::Cb, InstName::CSrli), // Mapped to srli: srli rd′, rd′, shamt
                    0x1 => (InstType::Cb, InstName::CSrai), // Mapped to srai: srai rd′, rd′, shamt
                    0x2 => (InstType::Cb, InstName::CAndi), // Mapped to andi: andi rd′, rd′, imm
                    0x3 => match (inst >> 12) & 0x1 {
                        0x0 => match (inst >> 5) & 0x3 {
                            0x0 => (InstType::Ca, InstName::CSub), // Mapped to sub: sub rd′, rd′, rs2′
                            0x1 => (InstType::Ca, InstName::CXor), // Mapped to xor: xor rd′, rd′, rs2′
                            0x2 => (InstType::Ca, InstName::COr), // Mapped to or: or rd′, rd′, rs2′
                            0x3 => (InstType::Ca, InstName::CAnd), // Mapped to and: and rd′, rd′, rs2′
                            _ => (InstType::Cinvalid, InstName::CReserved),
                        },
                        0x01 => match (inst >> 5) & 0x3 {
                            0x0 => (InstType::Ca, InstName::CSubw), // Mapped to subw: subw rd′, rd′, rs2′
                            0x1 => (InstType::Ca, InstName::CAddw), // Mapped to addw: addw rd′, rd′,rs2′
                            0x2 | 0x3 => (InstType::Cinvalid, InstName::CReserved),
                            _ => (InstType::Cinvalid, InstName::CReserved),
                        },
                        _ => (InstType::Cinvalid, InstName::CReserved),
                    },
                    _ => (InstType::Cinvalid, InstName::CReserved),
                },
                0x5 => (InstType::Cj, InstName::CJ), // Mapped to jal: jal x0, offset
                0x6 => (InstType::Cb, InstName::CBeqz), // Mapped to beq: beq rs1′, x0, offset
                0x7 => (InstType::Cb, InstName::CBnez), // Mapped to bne: bne rs1′, x0, offset
                _ => (InstType::Cinvalid, InstName::CReserved),
            },
            0x02 => {
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => (InstType::Ci, InstName::CSlli), // Mapped to slli: slli rd, rd, shamt[5:0]
                    0x1 => (InstType::Ci, InstName::CFldsp), // Mapped to ld: ld rd, offset(x2), rd!=0
                    // Would map to fld: fld rd, offset(x2), x2=sp, offset*8
                    0x2 => (InstType::Ci, InstName::CLwsp), // Mapped to lw: lw rd, offset(x2)
                    0x3 => (InstType::Ci, InstName::CLdsp), // Mapped to ld: ld rd, offset(x2), rd!=0
                    0x4 => {
                        match (inst >> 12) & 0x1 {
                            // Check bit 12
                            0x0 => {
                                match (inst >> 2) & 0x1F {
                                    // Check bits 6 to 2
                                    0x0 => (InstType::Cr, InstName::CJr), // Mapped to jalr: jalr x0, 0(rs1)
                                    _ => (InstType::Cr, InstName::CMv), // Mapped to add: add rd, x0, rs2
                                }
                            }
                            0x1 => {
                                match (inst >> 2) & 0x1F {
                                    // Check bits 6 to 2
                                    0x0 => {
                                        match (inst >> 7) & 0x1F {
                                            // Check bits 11 to 7
                                            0x0 => (InstType::Ci, InstName::CEbreak), // Mapped to ebreak
                                            _ => (InstType::Cr, InstName::CJalr), // Mapped to jalr: jalr x1, 0(rs1)
                                        }
                                    }
                                    _ => (InstType::Cr, InstName::CAdd), // Mapped to add: add rd, rd, rs2
                                }
                            }
                            _ => (InstType::Cinvalid, InstName::CReserved),
                        }
                    }
                    0x5 => (InstType::Css, InstName::CFsdsp), // Mapped to sd: sd rs2, offset(x2)
                    0x6 => (InstType::Css, InstName::CSwsp),  // Mapped to sw: sw rs2, offset(x2)
                    0x7 => (InstType::Css, InstName::CSdsp),  // Mapped to sd: sd rs2, offset(x2)
                    _ => (InstType::Cinvalid, InstName::CReserved),
                }
            }
            _ => (InstType::Cinvalid, InstName::CReserved),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_type_and_name_32_bits() {
        let mut instruction: u32;
        let mut result: (InstType, InstName, u64);

        // ========== OPCODE 3 - Load Instructions ==========
        instruction = 0x00010003; // lb x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Lb, 1));

        instruction = 0x00011003; // lh x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Lh, 1));

        instruction = 0x00012003; // lw x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Lw, 1));

        instruction = 0x00013003; // ld x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Ld, 1));

        instruction = 0x00014003; // lbu x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Lbu, 1));

        instruction = 0x00015003; // lhu x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Lhu, 1));

        instruction = 0x00016003; // lwu x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Lwu, 1));

        // ========== OPCODE 7 - Floating-point Load ==========
        instruction = 0x00012007; // flw f0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Flw, 1));

        instruction = 0x00013007; // fld f0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Fld, 1));

        // ========== OPCODE 15 - Fence Instructions ==========
        instruction = 0x0000000f; // fence
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::F, InstName::Fence, 1));

        instruction = 0x0000100f; // fence.i
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::F, InstName::FenceI, 1));

        // ========== OPCODE 19 - Immediate Arithmetic ==========
        instruction = 0x00010013; // addi x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Addi, 1));

        instruction = 0x00011013; // slli x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Slli, 2));

        instruction = 0x60411013; // sext.b x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::SextB, 2));

        instruction = 0x60511013; // sext.h x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::SextH, 2));

        instruction = 0x60011013; // clz x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Clz, 2));

        instruction = 0x60111013; // ctz x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Ctz, 2));

        instruction = 0x60211013; // cpop x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Cpop, 2));

        instruction = 0x28011013; // bseti x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Bseti, 2));

        instruction = 0x48011013; // bclri x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Bclri, 2));

        instruction = 0x68011013; // binvi x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Binvi, 2));

        instruction = 0x00012013; // slti x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Slti, 1));

        instruction = 0x00013013; // sltiu x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Sltiu, 1));

        instruction = 0x00014013; // xori x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Xori, 1));

        instruction = 0x00015013; // srli x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Srli, 2));

        instruction = 0x40015013; // srai x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Srai, 2));

        instruction = 0x48015013; // bexti x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Bexti, 2));

        instruction = 0x60015013; // rori x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Rori, 2));

        instruction = 0x6b815013; // rev8 x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Rev8, 2));

        instruction = 0x68715013; // brev8 x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Brev8, 2));

        instruction = 0x28715013; // orc.b x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::OrcB, 2));

        instruction = 0x00016013; // ori x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Ori, 1));

        instruction = 0x00017013; // andi x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Andi, 1));

        // ========== OPCODE 23 - AUIPC ==========
        instruction = 0x00000017; // auipc x0, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::U, InstName::Auipc, 0));

        // ========== OPCODE 27 - 32-bit Immediate Arithmetic ==========
        instruction = 0x0001001b; // addiw x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Addiw, 1));

        instruction = 0x6001101b; // clzw x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Clzw, 2));

        instruction = 0x6011101b; // ctzw x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Ctzw, 2));

        instruction = 0x6021101b; // cpopw x0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Cpopw, 2));

        instruction = 0x0801101b; // slli.uw x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::SlliUw, 2));

        instruction = 0x0001101b; // slliw x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Slliw, 2));

        instruction = 0x0001501b; // srliw x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Srliw, 2));

        instruction = 0x4001501b; // sraiw x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Sraiw, 2));

        instruction = 0x6001501b; // roriw x0, x2, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Roriw, 2));

        // ========== OPCODE 35 - Store Instructions ==========
        instruction = 0x00010023; // sb x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::S, InstName::Sb, 1));

        instruction = 0x00011023; // sh x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::S, InstName::Sh, 1));

        instruction = 0x00012023; // sw x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::S, InstName::Sw, 1));

        instruction = 0x00013023; // sd x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::S, InstName::Sd, 1));

        // ========== OPCODE 39 - Floating-point Store ==========
        instruction = 0x00012027; // fsw f0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::S, InstName::Fsw, 1));

        instruction = 0x00013027; // fsd f0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::S, InstName::Fsd, 1));

        // ========== OPCODE 47 - Atomic Instructions ==========
        instruction = 0x1001202f; // lr.w x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::LrW, 2));

        instruction = 0x1801202f; // sc.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::ScW, 2));

        instruction = 0x0801202f; // amoswap.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmoswapW, 2));

        instruction = 0x0001202f; // amoadd.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmoaddW, 2));

        instruction = 0x2001202f; // amoxor.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmoxorW, 2));

        instruction = 0x6001202f; // amoand.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmoandW, 2));

        instruction = 0x4001202f; // amoor.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmoorW, 2));

        instruction = 0x8001202f; // amomin.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmominW, 2));

        instruction = 0xa001202f; // amomax.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmomaxW, 2));

        instruction = 0xc001202f; // amominu.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmominuW, 2));

        instruction = 0xe001202f; // amomaxu.w x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmomaxuW, 2));

        instruction = 0x1001302f; // lr.d x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::LrD, 2));

        instruction = 0x1801302f; // sc.d x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::ScD, 2));

        instruction = 0x0801302f; // amoswap.d x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmoswapD, 2));

        instruction = 0x0001302f; // amoadd.d x0, x0, (x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::A, InstName::AmoaddD, 2));

        // ========== OPCODE 51 - Register-Register Arithmetic ==========
        instruction = 0x003100b3; // add x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Add, 2));

        instruction = 0x023100b3; // mul x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Mul, 2));

        instruction = 0x403100b3; // sub x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sub, 2));

        instruction = 0x003110b3; // sll x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sll, 2));

        instruction = 0x023110b3; // mulh x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Mulh, 2));

        instruction = 0x0a3110b3; // clmul x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Clmul, 2));

        instruction = 0x283110b3; // bset x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Bset, 2));

        instruction = 0x483110b3; // bclr x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Bclr, 2));

        instruction = 0x603110b3; // rol x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Rol, 2));

        instruction = 0x683110b3; // binv x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Binv, 2));

        instruction = 0x003120b3; // slt x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Slt, 2));

        instruction = 0x023120b3; // mulhsu x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Mulhsu, 2));

        instruction = 0x0a3120b3; // clmulr x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Clmulr, 2));

        instruction = 0x203120b3; // sh1add x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sh1add, 2));

        instruction = 0x283120b3; // xperm4 x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Xperm4, 2));

        instruction = 0x003130b3; // sltu x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sltu, 2));

        instruction = 0x023130b3; // mulhu x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Mulhu, 2));

        instruction = 0x0a3130b3; // clmulh x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Clmulh, 2));

        instruction = 0x003140b3; // xor x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Xor, 2));

        instruction = 0x023140b3; // div x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Div, 2));

        instruction = 0x083140b3; // pack x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Pack, 2));

        instruction = 0x0a3140b3; // min x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Min, 2));

        instruction = 0x203140b3; // sh2add x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sh2add, 2));

        instruction = 0x283140b3; // xperm8 x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Xperm8, 2));

        instruction = 0x403140b3; // xnor x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Xnor, 2));

        instruction = 0x003150b3; // srl x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Srl, 2));

        instruction = 0x023150b3; // divu x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Divu, 2));

        instruction = 0x0a3150b3; // minu x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Minu, 2));

        instruction = 0x403150b3; // sra x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sra, 2));

        instruction = 0x483150b3; // bext x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Bext, 2));

        instruction = 0x603150b3; // ror x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Ror, 2));

        instruction = 0x003160b3; // or x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Or, 2));

        instruction = 0x023160b3; // rem x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Rem, 2));

        instruction = 0x0a3160b3; // max x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Max, 2));

        instruction = 0x203160b3; // sh3add x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sh3add, 2));

        instruction = 0x403160b3; // orn x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Orn, 2));

        instruction = 0x003170b3; // and x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::And, 2));

        instruction = 0x023170b3; // remu x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Remu, 2));

        instruction = 0x083170b3; // packh x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Packh, 2));

        instruction = 0x0a3170b3; // maxu x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Maxu, 2));

        instruction = 0x403170b3; // andn x1, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Andn, 2));

        // ========== OPCODE 55 - LUI ==========
        instruction = 0x00000037; // lui x0, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::U, InstName::Lui, 0));

        // ========== OPCODE 59 - 32-bit Register-Register Arithmetic ==========
        instruction = 0x0031003b; // addw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Addw, 2));

        instruction = 0x0231003b; // mulw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Mulw, 2));

        instruction = 0x0831003b; // add.uw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::AddUw, 2));

        instruction = 0x4031003b; // subw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Subw, 2));

        instruction = 0x0031103b; // sllw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sllw, 2));

        instruction = 0x6031103b; // rolw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Rolw, 2));

        instruction = 0x2031203b; // sh1add.uw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sh1addUw, 2));

        instruction = 0x0800403b; // zext.h x0, x0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::ZextH, 2));

        instruction = 0x0231403b; // divw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Divw, 2));

        instruction = 0x0831403b; // packw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Packw, 2));

        instruction = 0x2031403b; // sh2add.uw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sh2addUw, 2));

        instruction = 0x0031503b; // srlw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Srlw, 2));

        instruction = 0x0231503b; // divuw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Divuw, 2));

        instruction = 0x4031503b; // sraw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sraw, 2));

        instruction = 0x6031503b; // rorw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Rorw, 2));

        instruction = 0x0231603b; // remw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Remw, 2));

        instruction = 0x2031603b; // sh3add.uw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Sh3addUw, 2));

        instruction = 0x0231703b; // remuw x0, x2, x3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::Remuw, 2));

        // ========== OPCODE 67, 71, 75, 79 - Floating-point Fused Multiply-Add ==========
        instruction = 0x00310043; // fmadd.s f0, f2, f3, f0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R4, InstName::FmaddS, 1));

        instruction = 0x02310043; // fmadd.d f0, f2, f3, f0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R4, InstName::FmaddD, 1));

        instruction = 0x00310047; // fmsub.s f0, f2, f3, f0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R4, InstName::FmsubS, 1));

        instruction = 0x0231004b; // fnmsub.d f0, f2, f3, f0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R4, InstName::FnmsubD, 1));

        instruction = 0x0031004f; // fnmadd.s f0, f2, f3, f0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R4, InstName::FnmaddS, 1));

        // ========== OPCODE 83 - Floating-point Arithmetic ==========
        instruction = 0x00310053; // fadd.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FaddS, 1));

        instruction = 0x02310053; // fadd.d f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FaddD, 1));

        instruction = 0x08310053; // fsub.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FsubS, 1));

        instruction = 0x0a310053; // fsub.d f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FsubD, 1));

        instruction = 0x10310053; // fmul.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FmulS, 1));

        instruction = 0x12310053; // fmul.d f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FmulD, 1));

        instruction = 0x18310053; // fdiv.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FdivS, 1));

        instruction = 0x1a310053; // fdiv.d f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FdivD, 1));

        instruction = 0x20310053; // fsgnj.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FsgnjS, 2));

        instruction = 0x20311053; // fsgnjn.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FsgnjnS, 2));

        instruction = 0x20312053; // fsgnjx.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FsgnjxS, 2));

        instruction = 0x22310053; // fsgnj.d f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FsgnjD, 2));

        instruction = 0x28310053; // fmin.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FminS, 2));

        instruction = 0x28311053; // fmax.s f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FmaxS, 2));

        instruction = 0x2a310053; // fmin.d f0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FminD, 2));

        instruction = 0x40110053; // fcvt.s.d f0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtSD, 2));

        instruction = 0x42010053; // fcvt.d.s f0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtDS, 2));

        instruction = 0x58010053; // fsqrt.s f0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FsqrtS, 2));

        instruction = 0x5a010053; // fsqrt.d f0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FsqrtD, 2));

        instruction = 0xa0312053; // feq.s x0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FeqS, 2));

        instruction = 0xa0311053; // flt.s x0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FltS, 2));

        instruction = 0xa0310053; // fle.s x0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FleS, 2));

        instruction = 0xa2312053; // feq.d x0, f2, f3
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FeqD, 2));

        instruction = 0xc0010053; // fcvt.w.s x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtWS, 2));

        instruction = 0xc0110053; // fcvt.wu.s x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtWuS, 2));

        instruction = 0xc0210053; // fcvt.l.s x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtLS, 2));

        instruction = 0xc0310053; // fcvt.lu.s x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtLuS, 2));

        instruction = 0xc2010053; // fcvt.w.d x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtWD, 2));

        instruction = 0xc2210053; // fcvt.l.d x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtLD, 2));

        instruction = 0xd0010053; // fcvt.s.w f0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtSW, 2));

        instruction = 0xd0110053; // fcvt.s.wu f0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtSWu, 2));

        instruction = 0xd0210053; // fcvt.s.l f0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtSL, 2));

        instruction = 0xd2010053; // fcvt.d.w f0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtDW, 2));

        instruction = 0xd2210053; // fcvt.d.l f0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FcvtDL, 2));

        instruction = 0xe0010053; // fmv.x.w x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FmvXW, 3));

        instruction = 0xe0011053; // fclass.s x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FclassS, 3));

        instruction = 0xe2010053; // fmv.x.d x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FmvXD, 3));

        instruction = 0xe2011053; // fclass.d x0, f2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::R, InstName::FclassD, 3));

        instruction = 0xf0010053; // fmv.w.x f0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::FmvWX, 3));

        instruction = 0xf2010053; // fmv.d.x f0, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::FmvDX, 3));

        // ========== OPCODE 99 - Branch Instructions ==========
        instruction = 0x00310063; // beq x2, x3, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::B, InstName::Beq, 1));

        instruction = 0x00311063; // bne x2, x3, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::B, InstName::Bne, 1));

        instruction = 0x00314063; // blt x2, x3, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::B, InstName::Blt, 1));

        instruction = 0x00315063; // bge x2, x3, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::B, InstName::Bge, 1));

        instruction = 0x00316063; // bltu x2, x3, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::B, InstName::Bltu, 1));

        instruction = 0x00317063; // bgeu x2, x3, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::B, InstName::Bgeu, 1));

        // ========== OPCODE 103 - JALR ==========
        instruction = 0x00010067; // jalr x0, 0(x2)
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::I, InstName::Jalr, 0));

        // ========== OPCODE 111 - JAL ==========
        instruction = 0x0000006f; // jal x0, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::J, InstName::Jal, 0));

        // ========== OPCODE 115 - System Instructions ==========
        instruction = 0x00000073; // ecall
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::C, InstName::Ecall, 2));

        instruction = 0x00100073; // ebreak
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::C, InstName::Ebreak, 2));

        instruction = 0x00011073; // csrrw x0, 0x001, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::C, InstName::Csrrw, 1));

        instruction = 0x00012073; // csrrs x0, 0x001, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::C, InstName::Csrrs, 1));

        instruction = 0x00013073; // csrrc x0, 0x001, x2
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::C, InstName::Csrrc, 1));

        instruction = 0x00015073; // csrrwi x0, 0x001, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::C, InstName::Csrrwi, 1));

        instruction = 0x00016073; // csrrsi x0, 0x001, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::C, InstName::Csrrsi, 1));

        instruction = 0x00017073; // csrrci x0, 0x001, 0
        result = RiscvDecoder::get_type_and_name_32_bits(instruction);
        assert_eq!(result, (InstType::C, InstName::Csrrci, 1));
    }

    #[test]
    fn test_get_type_and_name_16_bits() {
        let mut instruction: u16;
        let mut result: (InstType, InstName);

        // ========== OP2 = 0x00 ==========
        instruction = 0x0000; // reserved all-zero encoding
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cinvalid, InstName::CReserved));

        instruction = 0x0004; // c.addi4spn
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ciw, InstName::CAddi4spn));

        instruction = 0x2000; // c.fld
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cl, InstName::CFld));

        instruction = 0x4000; // c.lw
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cl, InstName::CLw));

        instruction = 0x6000; // c.ld
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cl, InstName::CLd));

        instruction = 0x8000; // reserved
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cinvalid, InstName::CReserved));

        instruction = 0xa000; // c.fsd
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cs, InstName::CFsd));

        instruction = 0xc000; // c.sw
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cs, InstName::CSw));

        instruction = 0xe000; // c.sd
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cs, InstName::CSd));

        // ========== OP2 = 0x01 ==========
        instruction = 0x0001; // c.nop
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CNop));

        instruction = 0x0081; // c.addi
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CAddi));

        instruction = 0x2001; // c.addiw
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CAddiw));

        instruction = 0x4001; // c.li
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CLi));

        instruction = 0x6101; // c.addi16sp
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CAddi16sp));

        instruction = 0x6181; // c.lui
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CLui));

        instruction = 0x8001; // c.srli
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cb, InstName::CSrli));

        instruction = 0x8401; // c.srai
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cb, InstName::CSrai));

        instruction = 0x8801; // c.andi
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cb, InstName::CAndi));

        instruction = 0x8c01; // c.sub
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ca, InstName::CSub));

        instruction = 0x8c21; // c.xor
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ca, InstName::CXor));

        instruction = 0x8c41; // c.or
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ca, InstName::COr));

        instruction = 0x8c61; // c.and
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ca, InstName::CAnd));

        instruction = 0x9c01; // c.subw
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ca, InstName::CSubw));

        instruction = 0x9c21; // c.addw
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ca, InstName::CAddw));

        instruction = 0x9c41; // reserved
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cinvalid, InstName::CReserved));

        instruction = 0xa001; // c.j
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cj, InstName::CJ));

        instruction = 0xc001; // c.beqz
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cb, InstName::CBeqz));

        instruction = 0xe001; // c.bnez
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cb, InstName::CBnez));

        // ========== OP2 = 0x02 ==========
        instruction = 0x0002; // c.slli
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CSlli));

        instruction = 0x2002; // c.fldsp
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CFldsp));

        instruction = 0x4002; // c.lwsp
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CLwsp));

        instruction = 0x6002; // c.ldsp
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CLdsp));

        instruction = 0x8002; // c.jr
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cr, InstName::CJr));

        instruction = 0x8006; // c.mv
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cr, InstName::CMv));

        instruction = 0x9002; // c.ebreak
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Ci, InstName::CEbreak));

        instruction = 0x9082; // c.jalr
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cr, InstName::CJalr));

        instruction = 0x9006; // c.add
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cr, InstName::CAdd));

        instruction = 0xa002; // c.fsdsp
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Css, InstName::CFsdsp));

        instruction = 0xc002; // c.swsp
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Css, InstName::CSwsp));

        instruction = 0xe002; // c.sdsp
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Css, InstName::CSdsp));

        // ========== Unknown OP2 ==========
        instruction = 0x0003;
        result = RiscvDecoder::get_type_and_name_16_bits(instruction);
        assert_eq!(result, (InstType::Cinvalid, InstName::CReserved));
    }
}
