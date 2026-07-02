//! RISC-V DECODER (RVD)
//!
//! Providing as a single argument a 32-bit or a 16-bit instruction, the RISC-V decoder returns
//! the instruction type and name, as well as the instruction level
//! (1, 2 or 3) for 32-bit instructions.
//!
//! The instruction type is a string, for example: I, S, B, U, J, R, R4, C, CIW, CL, CS, CA, CB or
//! CJ.  The instruction type is used to parse the instruction operands and immediate values in file
//! riscv_interpreter.rs.  It tells the interpreter what fields are present in the 32-bit (or 16-bit)
//! instruction, their position and length.  In other words, it tells the interpreter the meaning of
//! the instruction bits.
//!
//! The instruction name is the human-readable name of the instruction, e.g. "addi", "lw",
//! "c.addi4spn", etc., and it is used to transpile RISC-V to Zisk assembly in file
//! riscv2zisk_context.rs.
//!
//! For example: add x1, x2, x3 is encoded as a 32-bits instruction 0x003100b3, and after calling
//! Rvd::get_type_and_name_32_bits(0x003100b3) we get ("R", "add", 2) as a result.  With "R" we can
//! decode the values of rd, rs1 and rs2, and with "add" we can transpile it to Zisk assembly as
//! "add x1, x2, x3".

/// RVD structure
pub struct Rvd {}

/// RVD implementation
impl Rvd {
    pub fn get_type_and_name_32_bits(inst: u32) -> (&'static str, &'static str, u64) {
        match inst & 0x7F {
            3 => {
                // Opcode 3
                match (inst >> 12) & 0x7 {
                    0 => ("I", "lb", 1),
                    1 => ("I", "lh", 1),
                    2 => ("I", "lw", 1),
                    3 => ("I", "ld", 1),
                    4 => ("I", "lbu", 1),
                    5 => ("I", "lhu", 1),
                    6 => ("I", "lwu", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            7 => {
                // Opcode 7
                match (inst >> 12) & 0x7 {
                    0 => ("INVALID", "reserved", 1),
                    2 => ("I", "flw", 1),
                    3 => ("I", "fld", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            15 => {
                // Opcode 15
                match (inst >> 12) & 0x7 {
                    0 => ("F", "fence", 1),
                    1 => ("F", "fence.i", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            19 => {
                // Opcode 19
                match (inst >> 12) & 0x7 {
                    0 => ("I", "addi", 1),
                    1 => {
                        match (inst >> 20) & 0xFFF {
                            0b011000000100 => return ("I", "sext.b", 2),
                            0b011000000101 => return ("I", "sext.h", 2),
                            0b011000000000 => return ("I", "clz", 2),
                            0b011000000001 => return ("I", "ctz", 2),
                            0b011000000010 => return ("I", "cpop", 2),
                            _ => {}
                        }
                        match (inst >> 26) & 0x3F {
                            0 => ("I", "slli", 2),
                            10 => ("I", "bseti", 2),
                            18 => ("I", "bclri", 2),
                            26 => ("I", "binvi", 2),
                            _ => ("INVALID", "reserved", 2),
                        }
                    }
                    2 => ("I", "slti", 1),
                    3 => ("I", "sltiu", 1),
                    4 => ("I", "xori", 1),
                    5 => {
                        match (inst >> 20) & 0xFFF {
                            0b011010111000 => return ("I", "rev8", 2),
                            0b011010000111 => return ("I", "brev8", 2),
                            0b001010000111 => return ("I", "orc.b", 2),
                            _ => {}
                        }
                        match (inst >> 26) & 0x3F {
                            0 => ("I", "srli", 2),
                            16 => ("I", "srai", 2),
                            18 => ("I", "bexti", 2),
                            24 => ("I", "rori", 2),
                            _ => ("INVALID", "reserved", 2),
                        }
                    }
                    6 => ("I", "ori", 1),
                    7 => ("I", "andi", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            23 => {
                // Opcode 23
                ("U", "auipc", 0)
            }
            27 => {
                // Opcode 27
                match (inst >> 12) & 0x7 {
                    0 => ("I", "addiw", 1),
                    1 => {
                        match (inst >> 20) & 0xFFF {
                            0b011000000000 => return ("I", "clzw", 2),
                            0b011000000001 => return ("I", "ctzw", 2),
                            0b011000000010 => return ("I", "cpopw", 2),
                            _ => {}
                        }
                        if (inst >> 26) & 0x3F == 2 {
                            return ("I", "slli.uw", 2);
                        }
                        match (inst >> 25) & 0x7F {
                            0 => ("I", "slliw", 2),
                            _ => ("INVALID", "reserved", 2),
                        }
                    }
                    5 => match (inst >> 25) & 0x7F {
                        0 => ("I", "srliw", 2),
                        32 => ("I", "sraiw", 2),
                        48 => ("I", "roriw", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    _ => ("INVALID", "reserved", 1),
                }
            }
            35 => {
                // Opcode 35
                match (inst >> 12) & 0x7 {
                    0 => ("S", "sb", 1),
                    1 => ("S", "sh", 1),
                    2 => ("S", "sw", 1),
                    3 => ("S", "sd", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            39 =>
            // Opcode 39
            {
                match (inst >> 12) & 0x7 {
                    2 => ("S", "fsw", 1),
                    3 => ("S", "fsd", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            47 => {
                // Opcode 47
                match (inst >> 12) & 0x7 {
                    2 => match (inst >> 27) & 0x1F {
                        2 => ("A", "lr.w", 2),
                        3 => ("A", "sc.w", 2),
                        1 => ("A", "amoswap.w", 2),
                        0 => ("A", "amoadd.w", 2),
                        4 => ("A", "amoxor.w", 2),
                        12 => ("A", "amoand.w", 2),
                        8 => ("A", "amoor.w", 2),
                        16 => ("A", "amomin.w", 2),
                        20 => ("A", "amomax.w", 2),
                        24 => ("A", "amominu.w", 2),
                        28 => ("A", "amomaxu.w", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    3 => match (inst >> 27) & 0x1F {
                        2 => ("A", "lr.d", 2),
                        3 => ("A", "sc.d", 2),
                        1 => ("A", "amoswap.d", 2),
                        0 => ("A", "amoadd.d", 2),
                        4 => ("A", "amoxor.d", 2),
                        12 => ("A", "amoand.d", 2),
                        8 => ("A", "amoor.d", 2),
                        16 => ("A", "amomin.d", 2),
                        20 => ("A", "amomax.d", 2),
                        24 => ("A", "amominu.d", 2),
                        28 => ("A", "amomaxu.d", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    _ => ("INVALID", "reserved", 1),
                }
            }
            51 => {
                // Opcode 51
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 25) & 0x7F {
                        0 => ("R", "add", 2),
                        1 => ("R", "mul", 2),
                        32 => ("R", "sub", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    1 => match (inst >> 25) & 0x7F {
                        0 => ("R", "sll", 2),
                        1 => ("R", "mulh", 2),
                        5 => ("R", "clmul", 2),
                        20 => ("R", "bset", 2),
                        36 => ("R", "bclr", 2),
                        48 => ("R", "rol", 2),
                        52 => ("R", "binv", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    2 => match (inst >> 25) & 0x7F {
                        0 => ("R", "slt", 2),
                        1 => ("R", "mulhsu", 2),
                        5 => ("R", "clmulr", 2),
                        16 => ("R", "sh1add", 2),
                        20 => ("R", "xperm4", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    3 => match (inst >> 25) & 0x7F {
                        0 => ("R", "sltu", 2),
                        1 => ("R", "mulhu", 2),
                        5 => ("R", "clmulh", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    4 => match (inst >> 25) & 0x7F {
                        0 => ("R", "xor", 2),
                        1 => ("R", "div", 2),
                        4 => ("R", "pack", 2),
                        5 => ("R", "min", 2),
                        16 => ("R", "sh2add", 2),
                        20 => ("R", "xperm8", 2),
                        32 => ("R", "xnor", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    5 => match (inst >> 25) & 0x7F {
                        0 => ("R", "srl", 2),
                        1 => ("R", "divu", 2),
                        5 => ("R", "minu", 2),
                        32 => ("R", "sra", 2),
                        36 => ("R", "bext", 2),
                        48 => ("R", "ror", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    6 => match (inst >> 25) & 0x7F {
                        0 => ("R", "or", 2),
                        1 => ("R", "rem", 2),
                        5 => ("R", "max", 2),
                        16 => ("R", "sh3add", 2),
                        32 => ("R", "orn", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    7 => match (inst >> 25) & 0x7F {
                        0 => ("R", "and", 2),
                        1 => ("R", "remu", 2),
                        4 => ("R", "packh", 2),
                        5 => ("R", "maxu", 2),
                        32 => ("R", "andn", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    _ => ("INVALID", "reserved", 1),
                }
            }
            55 => {
                // Opcode 55
                ("U", "lui", 0)
            }
            59 => {
                // Opcode 59
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 25) & 0x7F {
                        0 => ("R", "addw", 2),
                        1 => ("R", "mulw", 2),
                        4 => ("R", "add.uw", 2),
                        32 => ("R", "subw", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    1 => match (inst >> 25) & 0x7F {
                        0 => ("R", "sllw", 2),
                        48 => ("R", "rolw", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    2 => match (inst >> 25) & 0x7F {
                        16 => ("R", "sh1add.uw", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    4 => {
                        if (inst >> 20) & 0xFFF == 0b000010000000 {
                            return ("R", "zext.h", 2);
                        }
                        match (inst >> 25) & 0x7F {
                            1 => ("R", "divw", 2),
                            4 => ("R", "packw", 2),
                            16 => ("R", "sh2add.uw", 2),
                            _ => ("INVALID", "reserved", 2),
                        }
                    }
                    5 => match (inst >> 25) & 0x7F {
                        0 => ("R", "srlw", 2),
                        1 => ("R", "divuw", 2),
                        32 => ("R", "sraw", 2),
                        48 => ("R", "rorw", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    6 => match (inst >> 25) & 0x7F {
                        1 => ("R", "remw", 2),
                        16 => ("R", "sh3add.uw", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    7 => match (inst >> 25) & 0x7F {
                        1 => ("R", "remuw", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    _ => ("INVALID", "reserved", 1),
                }
            }
            67 => {
                // Opcode 67
                match (inst >> 25) & 0x3 {
                    0 => ("R4", "fmadd.s", 1),
                    1 => ("R4", "fmadd.d", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            71 => {
                // Opcode 71
                match (inst >> 25) & 0x3 {
                    0 => ("R4", "fmsub.s", 1),
                    1 => ("R4", "fmsub.d", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            75 => {
                // Opcode 75
                match (inst >> 25) & 0x3 {
                    0 => ("R4", "fnmsub.s", 1),
                    1 => ("R4", "fnmsub.d", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            79 => {
                // Opcode 79
                match (inst >> 25) & 0x3 {
                    0 => ("R4", "fnmadd.s", 1),
                    1 => ("R4", "fnmadd.d", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            83 => {
                // Opcode 83
                match (inst >> 25) & 0x7F {
                    0 => ("R", "fadd.s", 1),
                    1 => ("R", "fadd.d", 1),
                    4 => ("R", "fsub.s", 1),
                    5 => ("R", "fsub.d", 1),
                    8 => ("R", "fmul.s", 1),
                    9 => ("R", "fmul.d", 1),
                    12 => ("R", "fdiv.s", 1),
                    13 => ("R", "fdiv.d", 1),
                    16 => match (inst >> 12) & 0x7 {
                        0 => ("R", "fsgnj.s", 2),
                        1 => ("R", "fsgnjn.s", 2),
                        2 => ("R", "fsgnjx.s", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    17 => match (inst >> 12) & 0x7 {
                        0 => ("R", "fsgnj.d", 2),
                        1 => ("R", "fsgnjn.d", 2),
                        2 => ("R", "fsgnjx.d", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    20 => match (inst >> 12) & 0x7 {
                        0 => ("R", "fmin.s", 2),
                        1 => ("R", "fmax.s", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    21 => match (inst >> 12) & 0x7 {
                        0 => ("R", "fmin.d", 2),
                        1 => ("R", "fmax.d", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    32 => match (inst >> 20) & 0x1F {
                        1 => ("R", "fcvt.s.d", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    33 => match (inst >> 20) & 0x1F {
                        0 => ("R", "fcvt.d.s", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    44 => match (inst >> 20) & 0x1F {
                        0 => ("R", "fsqrt.s", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    45 => match (inst >> 20) & 0x1F {
                        0 => ("R", "fsqrt.d", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    80 => match (inst >> 12) & 0x7 {
                        2 => ("R", "feq.s", 2),
                        1 => ("R", "flt.s", 2),
                        0 => ("R", "fle.s", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    81 => match (inst >> 12) & 0x7 {
                        2 => ("R", "feq.d", 2),
                        1 => ("R", "flt.d", 2),
                        0 => ("R", "fle.d", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    96 => match (inst >> 20) & 0x1F {
                        0 => ("R", "fcvt.w.s", 2),
                        1 => ("R", "fcvt.wu.s", 2),
                        2 => ("R", "fcvt.l.s", 2),
                        3 => ("R", "fcvt.lu.s", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    97 => match (inst >> 20) & 0x1F {
                        0 => ("R", "fcvt.w.d", 2),
                        1 => ("R", "fcvt.wu.d", 2),
                        2 => ("R", "fcvt.l.d", 2),
                        3 => ("R", "fcvt.lu.d", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    104 => match (inst >> 20) & 0x1F {
                        0 => ("R", "fcvt.s.w", 2),
                        1 => ("R", "fcvt.s.wu", 2),
                        2 => ("R", "fcvt.s.l", 2),
                        3 => ("R", "fcvt.s.lu", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    105 => match (inst >> 20) & 0x1F {
                        0 => ("R", "fcvt.d.w", 2),
                        1 => ("R", "fcvt.d.wu", 2),
                        2 => ("R", "fcvt.d.l", 2),
                        3 => ("R", "fcvt.d.lu", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    112 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => ("R", "fmv.x.w", 3),
                            _ => ("INVALID", "reserved", 3),
                        },
                        1 => match (inst >> 20) & 0x1F {
                            0 => ("R", "fclass.s", 3),
                            _ => ("INVALID", "reserved", 3),
                        },
                        _ => ("INVALID", "reserved", 2),
                    },
                    113 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => ("R", "fmv.x.d", 3),
                            _ => ("INVALID", "reserved", 3),
                        },
                        1 => match (inst >> 20) & 0x1F {
                            0 => ("R", "fclass.d", 3),
                            _ => ("INVALID", "reserved", 3),
                        },
                        _ => ("INVALID", "reserved", 2),
                    },
                    120 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => ("I", "fmv.w.x", 3),
                            _ => ("INVALID", "reserved", 3),
                        },
                        _ => ("INVALID", "reserved", 2),
                    },
                    121 => match (inst >> 12) & 0x7 {
                        0 => match (inst >> 20) & 0x1F {
                            0 => ("I", "fmv.d.x", 3),
                            _ => ("INVALID", "reserved", 3),
                        },
                        _ => ("INVALID", "reserved", 2),
                    },
                    _ => ("INVALID", "reserved", 1),
                }
            }
            99 => {
                // Opcode 99
                match (inst >> 12) & 0x7 {
                    0 => ("B", "beq", 1),
                    1 => ("B", "bne", 1),
                    4 => ("B", "blt", 1),
                    5 => ("B", "bge", 1),
                    6 => ("B", "bltu", 1),
                    7 => ("B", "bgeu", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            103 => {
                // Opcode 103
                ("I", "jalr", 0)
            }
            111 => {
                // Opcode 111
                ("J", "jal", 0)
            }
            115 => {
                // Opcode 115
                match (inst >> 12) & 0x7 {
                    0 => match (inst >> 20) & 0xFFF {
                        0 => ("C", "ecall", 2),
                        1 => ("C", "ebreak", 2),
                        _ => ("INVALID", "reserved", 2),
                    },
                    1 => ("C", "csrrw", 1),
                    2 => ("C", "csrrs", 1),
                    3 => ("C", "csrrc", 1),
                    5 => ("C", "csrrwi", 1),
                    6 => ("C", "csrrsi", 1),
                    7 => ("C", "csrrci", 1),
                    _ => ("INVALID", "reserved", 1),
                }
            }
            _ => ("INVALID", "reserved", 0),
        }
    }

    // Converts a compressed register index (e.g. rs1') to a full register index (e.g. rs1)
    // Source: https://www2.eecs.berkeley.edu/Pubs/TechRpts/2015/EECS-2015-209.pdf
    //     RVC Register Number 000 001 010 011 100 101 110 111
    // Integer Register Number  x8  x9 x10 x11 x12 x13 x14 x15
    pub fn convert_compressed_reg_index(reg: u32) -> u32 {
        match reg {
            0 => 8,  // x8
            1 => 9,  // x9
            2 => 10, // x10
            3 => 11, // x11
            4 => 12, // x12
            5 => 13, // x13
            6 => 14, // x14
            7 => 15, // x15
            _ => panic!(
                "Rvd::convert_compressed_reg_index() invalid compressed register index {}",
                reg
            ),
        }
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

    pub fn get_type_and_name_16_bits(inst: u16) -> (&'static str, &'static str) {
        //println!("Rvd::get_type_and_name_16_bits() inst=0x{:x}", inst);
        // Return the type and name of the instruction
        match inst & 0x3 {
            // Check bits 1 and 0 = op2
            0x00 => {
                if inst == 0x0000 {
                    return ("CINVALID", "c.reserved");
                }
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => ("CIW", "c.addi4spn"), // Mapped to addi: addi rd′, x2, nzuimm[9:2]
                    0x1 => ("CL", "c.fld"),       // Mapped to ld: ld rd′, offset(rs1′)
                    0x2 => ("CL", "c.lw"),        // Mapped to lw: lw rd′, offset(rs1′)
                    0x3 => ("CL", "c.ld"),        // Mapped to ld: ld rd′, offset(rs1′)
                    0x4 => ("CINVALID", "c.reserved"), // Reserved
                    0x5 => ("CS", "c.fsd"),       // Mapped to sd: sd rs2′, offset(rs1′)
                    0x6 => ("CS", "c.sw"),        // Mapped to sw: sw rs2′,offset(rs1′)
                    0x7 => ("CS", "c.sd"),        // Mapped to sd: sd rs2′, offset(rs1′)
                    _ => ("CINVALID", "c.reserved"),
                }
            }
            0x01 => match (inst >> 13) & 0x7 {
                // Check bits 15 to 13 = funct3
                0x0 => {
                    if ((inst >> 7) & 0x1F) == 0x0 {
                        ("CI", "c.nop") // Transpiled to ZisK nop (flag)
                    } else {
                        ("CI", "c.addi") // Mapped to addi: addi rd, rd, imm
                    }
                }
                0x1 => ("CI", "c.addiw"), // Mapped to addiw: addiw rd, rd, imm
                0x2 => ("CI", "c.li"),    // Mapped to addi: addi rd, x0, imm
                0x3 => {
                    if ((inst >> 7) & 0x1F) == 2 {
                        ("CI", "c.addi16sp") // Mapped to addi: addi x2, x2, nzimm[9:4]
                    } else {
                        ("CI", "c.lui") // Mapped to lui: lui rd, imm
                    }
                }
                0x4 => match (inst >> 10) & 0x3 {
                    0x0 => ("CB", "c.srli"), // Mapped to srli: srli rd′, rd′, shamt
                    0x1 => ("CB", "c.srai"), // Mapped to srai: srai rd′, rd′, shamt
                    0x2 => ("CB", "c.andi"), // Mapped to andi: andi rd′, rd′, imm
                    0x3 => match (inst >> 12) & 0x1 {
                        0x0 => match (inst >> 5) & 0x3 {
                            0x0 => ("CA", "c.sub"), // Mapped to sub: sub rd′, rd′, rs2′
                            0x1 => ("CA", "c.xor"), // Mapped to xor: xor rd′, rd′, rs2′
                            0x2 => ("CA", "c.or"),  // Mapped to or: or rd′, rd′, rs2′
                            0x3 => ("CA", "c.and"), // Mapped to and: and rd′, rd′, rs2′
                            _ => ("CINVALID", "c.reserved"),
                        },
                        0x01 => match (inst >> 5) & 0x3 {
                            0x0 => ("CA", "c.subw"), // Mapped to subw: subw rd′, rd′, rs2′
                            0x1 => ("CA", "c.addw"), // Mapped to addw: addw rd′, rd′,rs2′
                            0x2 | 0x3 => ("CINVALID", "c.reserved"),
                            _ => ("CINVALID", "c.reserved"),
                        },
                        _ => ("CINVALID", "c.reserved"),
                    },
                    _ => ("CINVALID", "c.reserved"),
                },
                0x5 => ("CJ", "c.j"),    // Mapped to jal: jal x0, offset
                0x6 => ("CB", "c.beqz"), // Mapped to beq: beq rs1′, x0, offset
                0x7 => ("CB", "c.bnez"), // Mapped to bne: bne rs1′, x0, offset
                _ => ("CINVALID", "c.reserved"),
            },
            0x02 => {
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => ("CI", "c.slli"), // Mapped to slli: slli rd, rd, shamt[5:0]
                    0x1 => ("CI", "c.fldsp"), // Mapped to ld: ld rd, offset(x2), rd!=0
                    // Would map to fld: fld rd, offset(x2), x2=sp, offset*8
                    0x2 => ("CI", "c.lwsp"), // Mapped to lw: lw rd, offset(x2)
                    0x3 => ("CI", "c.ldsp"), // Mapped to ld: ld rd, offset(x2), rd!=0
                    0x4 => {
                        match (inst >> 12) & 0x1 {
                            // Check bit 12
                            0x0 => {
                                match (inst >> 2) & 0x1F {
                                    // Check bits 6 to 2
                                    0x0 => ("CR", "c.jr"), // Mapped to jalr: jalr x0, 0(rs1)
                                    _ => ("CR", "c.mv"),   // Mapped to add: add rd, x0, rs2
                                }
                            }
                            0x1 => {
                                match (inst >> 2) & 0x1F {
                                    // Check bits 6 to 2
                                    0x0 => {
                                        match (inst >> 7) & 0x1F {
                                            // Check bits 11 to 7
                                            0x0 => ("CI", "c.ebreak"), // Mapped to ebreak
                                            _ => ("CR", "c.jalr"), // Mapped to jalr: jalr x1, 0(rs1)
                                        }
                                    }
                                    _ => ("CR", "c.add"), // Mapped to add: add rd, rd, rs2
                                }
                            }
                            _ => ("CINVALID", "c.reserved"),
                        }
                    }
                    0x5 => ("CSS", "c.fsdsp"), // Mapped to sd: sd rs2, offset(x2)
                    0x6 => ("CSS", "c.swsp"),  // Mapped to sw: sw rs2, offset(x2)
                    0x7 => ("CSS", "c.sdsp"),  // Mapped to sd: sd rs2, offset(x2)
                    _ => ("CINVALID", "c.reserved"),
                }
            }
            _ => ("CINVALID", "c.reserved"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_type_and_name_32_bits() {
        let mut instruction: u32;
        let mut result: (&str, &str, u64);

        // ========== OPCODE 3 - Load Instructions ==========
        instruction = 0x00010003; // lb x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "lb", 1));

        instruction = 0x00011003; // lh x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "lh", 1));

        instruction = 0x00012003; // lw x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "lw", 1));

        instruction = 0x00013003; // ld x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "ld", 1));

        instruction = 0x00014003; // lbu x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "lbu", 1));

        instruction = 0x00015003; // lhu x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "lhu", 1));

        instruction = 0x00016003; // lwu x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "lwu", 1));

        // ========== OPCODE 7 - Floating-point Load ==========
        instruction = 0x00012007; // flw f0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "flw", 1));

        instruction = 0x00013007; // fld f0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "fld", 1));

        // ========== OPCODE 15 - Fence Instructions ==========
        instruction = 0x0000000f; // fence
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("F", "fence", 1));

        instruction = 0x0000100f; // fence.i
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("F", "fence.i", 1));

        // ========== OPCODE 19 - Immediate Arithmetic ==========
        instruction = 0x00010013; // addi x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "addi", 1));

        instruction = 0x00011013; // slli x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "slli", 2));

        instruction = 0x60411013; // sext.b x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "sext.b", 2));

        instruction = 0x60511013; // sext.h x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "sext.h", 2));

        instruction = 0x60011013; // clz x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "clz", 2));

        instruction = 0x60111013; // ctz x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "ctz", 2));

        instruction = 0x60211013; // cpop x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "cpop", 2));

        instruction = 0x28011013; // bseti x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "bseti", 2));

        instruction = 0x48011013; // bclri x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "bclri", 2));

        instruction = 0x68011013; // binvi x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "binvi", 2));

        instruction = 0x00012013; // slti x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "slti", 1));

        instruction = 0x00013013; // sltiu x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "sltiu", 1));

        instruction = 0x00014013; // xori x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "xori", 1));

        instruction = 0x00015013; // srli x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "srli", 2));

        instruction = 0x40015013; // srai x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "srai", 2));

        instruction = 0x48015013; // bexti x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "bexti", 2));

        instruction = 0x60015013; // rori x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "rori", 2));

        instruction = 0x6b815013; // rev8 x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "rev8", 2));

        instruction = 0x68715013; // brev8 x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "brev8", 2));

        instruction = 0x28715013; // orc.b x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "orc.b", 2));

        instruction = 0x00016013; // ori x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "ori", 1));

        instruction = 0x00017013; // andi x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "andi", 1));

        // ========== OPCODE 23 - AUIPC ==========
        instruction = 0x00000017; // auipc x0, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("U", "auipc", 0));

        // ========== OPCODE 27 - 32-bit Immediate Arithmetic ==========
        instruction = 0x0001001b; // addiw x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "addiw", 1));

        instruction = 0x6001101b; // clzw x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "clzw", 2));

        instruction = 0x6011101b; // ctzw x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "ctzw", 2));

        instruction = 0x6021101b; // cpopw x0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "cpopw", 2));

        instruction = 0x0801101b; // slli.uw x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "slli.uw", 2));

        instruction = 0x0001101b; // slliw x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "slliw", 2));

        instruction = 0x0001501b; // srliw x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "srliw", 2));

        instruction = 0x4001501b; // sraiw x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "sraiw", 2));

        instruction = 0x6001501b; // roriw x0, x2, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "roriw", 2));

        // ========== OPCODE 35 - Store Instructions ==========
        instruction = 0x00010023; // sb x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("S", "sb", 1));

        instruction = 0x00011023; // sh x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("S", "sh", 1));

        instruction = 0x00012023; // sw x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("S", "sw", 1));

        instruction = 0x00013023; // sd x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("S", "sd", 1));

        // ========== OPCODE 39 - Floating-point Store ==========
        instruction = 0x00012027; // fsw f0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("S", "fsw", 1));

        instruction = 0x00013027; // fsd f0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("S", "fsd", 1));

        // ========== OPCODE 47 - Atomic Instructions ==========
        instruction = 0x1001202f; // lr.w x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "lr.w", 2));

        instruction = 0x1801202f; // sc.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "sc.w", 2));

        instruction = 0x0801202f; // amoswap.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amoswap.w", 2));

        instruction = 0x0001202f; // amoadd.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amoadd.w", 2));

        instruction = 0x2001202f; // amoxor.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amoxor.w", 2));

        instruction = 0x6001202f; // amoand.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amoand.w", 2));

        instruction = 0x4001202f; // amoor.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amoor.w", 2));

        instruction = 0x8001202f; // amomin.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amomin.w", 2));

        instruction = 0xa001202f; // amomax.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amomax.w", 2));

        instruction = 0xc001202f; // amominu.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amominu.w", 2));

        instruction = 0xe001202f; // amomaxu.w x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amomaxu.w", 2));

        instruction = 0x1001302f; // lr.d x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "lr.d", 2));

        instruction = 0x1801302f; // sc.d x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "sc.d", 2));

        instruction = 0x0801302f; // amoswap.d x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amoswap.d", 2));

        instruction = 0x0001302f; // amoadd.d x0, x0, (x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("A", "amoadd.d", 2));

        // ========== OPCODE 51 - Register-Register Arithmetic ==========
        instruction = 0x003100b3; // add x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "add", 2));

        instruction = 0x023100b3; // mul x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "mul", 2));

        instruction = 0x403100b3; // sub x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sub", 2));

        instruction = 0x003110b3; // sll x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sll", 2));

        instruction = 0x023110b3; // mulh x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "mulh", 2));

        instruction = 0x0a3110b3; // clmul x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "clmul", 2));

        instruction = 0x283110b3; // bset x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "bset", 2));

        instruction = 0x483110b3; // bclr x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "bclr", 2));

        instruction = 0x603110b3; // rol x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "rol", 2));

        instruction = 0x683110b3; // binv x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "binv", 2));

        instruction = 0x003120b3; // slt x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "slt", 2));

        instruction = 0x023120b3; // mulhsu x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "mulhsu", 2));

        instruction = 0x0a3120b3; // clmulr x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "clmulr", 2));

        instruction = 0x203120b3; // sh1add x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sh1add", 2));

        instruction = 0x283120b3; // xperm4 x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "xperm4", 2));

        instruction = 0x003130b3; // sltu x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sltu", 2));

        instruction = 0x023130b3; // mulhu x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "mulhu", 2));

        instruction = 0x0a3130b3; // clmulh x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "clmulh", 2));

        instruction = 0x003140b3; // xor x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "xor", 2));

        instruction = 0x023140b3; // div x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "div", 2));

        instruction = 0x083140b3; // pack x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "pack", 2));

        instruction = 0x0a3140b3; // min x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "min", 2));

        instruction = 0x203140b3; // sh2add x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sh2add", 2));

        instruction = 0x283140b3; // xperm8 x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "xperm8", 2));

        instruction = 0x403140b3; // xnor x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "xnor", 2));

        instruction = 0x003150b3; // srl x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "srl", 2));

        instruction = 0x023150b3; // divu x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "divu", 2));

        instruction = 0x0a3150b3; // minu x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "minu", 2));

        instruction = 0x403150b3; // sra x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sra", 2));

        instruction = 0x483150b3; // bext x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "bext", 2));

        instruction = 0x603150b3; // ror x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "ror", 2));

        instruction = 0x003160b3; // or x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "or", 2));

        instruction = 0x023160b3; // rem x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "rem", 2));

        instruction = 0x0a3160b3; // max x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "max", 2));

        instruction = 0x203160b3; // sh3add x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sh3add", 2));

        instruction = 0x403160b3; // orn x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "orn", 2));

        instruction = 0x003170b3; // and x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "and", 2));

        instruction = 0x023170b3; // remu x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "remu", 2));

        instruction = 0x083170b3; // packh x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "packh", 2));

        instruction = 0x0a3170b3; // maxu x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "maxu", 2));

        instruction = 0x403170b3; // andn x1, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "andn", 2));

        // ========== OPCODE 55 - LUI ==========
        instruction = 0x00000037; // lui x0, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("U", "lui", 0));

        // ========== OPCODE 59 - 32-bit Register-Register Arithmetic ==========
        instruction = 0x0031003b; // addw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "addw", 2));

        instruction = 0x0231003b; // mulw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "mulw", 2));

        instruction = 0x0831003b; // add.uw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "add.uw", 2));

        instruction = 0x4031003b; // subw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "subw", 2));

        instruction = 0x0031103b; // sllw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sllw", 2));

        instruction = 0x6031103b; // rolw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "rolw", 2));

        instruction = 0x2031203b; // sh1add.uw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sh1add.uw", 2));

        instruction = 0x0800403b; // zext.h x0, x0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "zext.h", 2));

        instruction = 0x0231403b; // divw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "divw", 2));

        instruction = 0x0831403b; // packw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "packw", 2));

        instruction = 0x2031403b; // sh2add.uw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sh2add.uw", 2));

        instruction = 0x0031503b; // srlw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "srlw", 2));

        instruction = 0x0231503b; // divuw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "divuw", 2));

        instruction = 0x4031503b; // sraw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sraw", 2));

        instruction = 0x6031503b; // rorw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "rorw", 2));

        instruction = 0x0231603b; // remw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "remw", 2));

        instruction = 0x2031603b; // sh3add.uw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "sh3add.uw", 2));

        instruction = 0x0231703b; // remuw x0, x2, x3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "remuw", 2));

        // ========== OPCODE 67, 71, 75, 79 - Floating-point Fused Multiply-Add ==========
        instruction = 0x00310043; // fmadd.s f0, f2, f3, f0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R4", "fmadd.s", 1));

        instruction = 0x02310043; // fmadd.d f0, f2, f3, f0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R4", "fmadd.d", 1));

        instruction = 0x00310047; // fmsub.s f0, f2, f3, f0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R4", "fmsub.s", 1));

        instruction = 0x0231004b; // fnmsub.d f0, f2, f3, f0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R4", "fnmsub.d", 1));

        instruction = 0x0031004f; // fnmadd.s f0, f2, f3, f0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R4", "fnmadd.s", 1));

        // ========== OPCODE 83 - Floating-point Arithmetic ==========
        instruction = 0x00310053; // fadd.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fadd.s", 1));

        instruction = 0x02310053; // fadd.d f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fadd.d", 1));

        instruction = 0x08310053; // fsub.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fsub.s", 1));

        instruction = 0x0a310053; // fsub.d f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fsub.d", 1));

        instruction = 0x10310053; // fmul.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fmul.s", 1));

        instruction = 0x12310053; // fmul.d f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fmul.d", 1));

        instruction = 0x18310053; // fdiv.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fdiv.s", 1));

        instruction = 0x1a310053; // fdiv.d f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fdiv.d", 1));

        instruction = 0x20310053; // fsgnj.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fsgnj.s", 2));

        instruction = 0x20311053; // fsgnjn.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fsgnjn.s", 2));

        instruction = 0x20312053; // fsgnjx.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fsgnjx.s", 2));

        instruction = 0x22310053; // fsgnj.d f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fsgnj.d", 2));

        instruction = 0x28310053; // fmin.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fmin.s", 2));

        instruction = 0x28311053; // fmax.s f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fmax.s", 2));

        instruction = 0x2a310053; // fmin.d f0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fmin.d", 2));

        instruction = 0x40110053; // fcvt.s.d f0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.s.d", 2));

        instruction = 0x42010053; // fcvt.d.s f0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.d.s", 2));

        instruction = 0x58010053; // fsqrt.s f0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fsqrt.s", 2));

        instruction = 0x5a010053; // fsqrt.d f0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fsqrt.d", 2));

        instruction = 0xa0312053; // feq.s x0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "feq.s", 2));

        instruction = 0xa0311053; // flt.s x0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "flt.s", 2));

        instruction = 0xa0310053; // fle.s x0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fle.s", 2));

        instruction = 0xa2312053; // feq.d x0, f2, f3
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "feq.d", 2));

        instruction = 0xc0010053; // fcvt.w.s x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.w.s", 2));

        instruction = 0xc0110053; // fcvt.wu.s x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.wu.s", 2));

        instruction = 0xc0210053; // fcvt.l.s x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.l.s", 2));

        instruction = 0xc0310053; // fcvt.lu.s x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.lu.s", 2));

        instruction = 0xc2010053; // fcvt.w.d x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.w.d", 2));

        instruction = 0xc2210053; // fcvt.l.d x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.l.d", 2));

        instruction = 0xd0010053; // fcvt.s.w f0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.s.w", 2));

        instruction = 0xd0110053; // fcvt.s.wu f0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.s.wu", 2));

        instruction = 0xd0210053; // fcvt.s.l f0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.s.l", 2));

        instruction = 0xd2010053; // fcvt.d.w f0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.d.w", 2));

        instruction = 0xd2210053; // fcvt.d.l f0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fcvt.d.l", 2));

        instruction = 0xe0010053; // fmv.x.w x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fmv.x.w", 3));

        instruction = 0xe0011053; // fclass.s x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fclass.s", 3));

        instruction = 0xe2010053; // fmv.x.d x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fmv.x.d", 3));

        instruction = 0xe2011053; // fclass.d x0, f2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("R", "fclass.d", 3));

        instruction = 0xf0010053; // fmv.w.x f0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "fmv.w.x", 3));

        instruction = 0xf2010053; // fmv.d.x f0, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "fmv.d.x", 3));

        // ========== OPCODE 99 - Branch Instructions ==========
        instruction = 0x00310063; // beq x2, x3, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("B", "beq", 1));

        instruction = 0x00311063; // bne x2, x3, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("B", "bne", 1));

        instruction = 0x00314063; // blt x2, x3, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("B", "blt", 1));

        instruction = 0x00315063; // bge x2, x3, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("B", "bge", 1));

        instruction = 0x00316063; // bltu x2, x3, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("B", "bltu", 1));

        instruction = 0x00317063; // bgeu x2, x3, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("B", "bgeu", 1));

        // ========== OPCODE 103 - JALR ==========
        instruction = 0x00010067; // jalr x0, 0(x2)
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("I", "jalr", 0));

        // ========== OPCODE 111 - JAL ==========
        instruction = 0x0000006f; // jal x0, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("J", "jal", 0));

        // ========== OPCODE 115 - System Instructions ==========
        instruction = 0x00000073; // ecall
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("C", "ecall", 2));

        instruction = 0x00100073; // ebreak
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("C", "ebreak", 2));

        instruction = 0x00011073; // csrrw x0, 0x001, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("C", "csrrw", 1));

        instruction = 0x00012073; // csrrs x0, 0x001, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("C", "csrrs", 1));

        instruction = 0x00013073; // csrrc x0, 0x001, x2
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("C", "csrrc", 1));

        instruction = 0x00015073; // csrrwi x0, 0x001, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("C", "csrrwi", 1));

        instruction = 0x00016073; // csrrsi x0, 0x001, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("C", "csrrsi", 1));

        instruction = 0x00017073; // csrrci x0, 0x001, 0
        result = Rvd::get_type_and_name_32_bits(instruction);
        assert_eq!(result, ("C", "csrrci", 1));
    }

    #[test]
    fn test_get_type_and_name_16_bits() {
        let mut instruction: u16;
        let mut result: (&str, &str);

        // ========== OP2 = 0x00 ==========
        instruction = 0x0000; // reserved all-zero encoding
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CINVALID", "c.reserved"));

        instruction = 0x0004; // c.addi4spn
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CIW", "c.addi4spn"));

        instruction = 0x2000; // c.fld
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CL", "c.fld"));

        instruction = 0x4000; // c.lw
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CL", "c.lw"));

        instruction = 0x6000; // c.ld
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CL", "c.ld"));

        instruction = 0x8000; // reserved
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CINVALID", "c.reserved"));

        instruction = 0xa000; // c.fsd
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CS", "c.fsd"));

        instruction = 0xc000; // c.sw
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CS", "c.sw"));

        instruction = 0xe000; // c.sd
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CS", "c.sd"));

        // ========== OP2 = 0x01 ==========
        instruction = 0x0001; // c.nop
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.nop"));

        instruction = 0x0081; // c.addi
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.addi"));

        instruction = 0x2001; // c.addiw
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.addiw"));

        instruction = 0x4001; // c.li
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.li"));

        instruction = 0x6101; // c.addi16sp
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.addi16sp"));

        instruction = 0x6181; // c.lui
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.lui"));

        instruction = 0x8001; // c.srli
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CB", "c.srli"));

        instruction = 0x8401; // c.srai
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CB", "c.srai"));

        instruction = 0x8801; // c.andi
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CB", "c.andi"));

        instruction = 0x8c01; // c.sub
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CA", "c.sub"));

        instruction = 0x8c21; // c.xor
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CA", "c.xor"));

        instruction = 0x8c41; // c.or
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CA", "c.or"));

        instruction = 0x8c61; // c.and
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CA", "c.and"));

        instruction = 0x9c01; // c.subw
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CA", "c.subw"));

        instruction = 0x9c21; // c.addw
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CA", "c.addw"));

        instruction = 0x9c41; // reserved
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CINVALID", "c.reserved"));

        instruction = 0xa001; // c.j
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CJ", "c.j"));

        instruction = 0xc001; // c.beqz
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CB", "c.beqz"));

        instruction = 0xe001; // c.bnez
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CB", "c.bnez"));

        // ========== OP2 = 0x02 ==========
        instruction = 0x0002; // c.slli
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.slli"));

        instruction = 0x2002; // c.fldsp
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.fldsp"));

        instruction = 0x4002; // c.lwsp
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.lwsp"));

        instruction = 0x6002; // c.ldsp
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.ldsp"));

        instruction = 0x8002; // c.jr
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CR", "c.jr"));

        instruction = 0x8006; // c.mv
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CR", "c.mv"));

        instruction = 0x9002; // c.ebreak
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CI", "c.ebreak"));

        instruction = 0x9082; // c.jalr
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CR", "c.jalr"));

        instruction = 0x9006; // c.add
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CR", "c.add"));

        instruction = 0xa002; // c.fsdsp
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CSS", "c.fsdsp"));

        instruction = 0xc002; // c.swsp
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CSS", "c.swsp"));

        instruction = 0xe002; // c.sdsp
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CSS", "c.sdsp"));

        // ========== Unknown OP2 ==========
        instruction = 0x0003;
        result = Rvd::get_type_and_name_16_bits(instruction);
        assert_eq!(result, ("CINVALID", "c.reserved"));
    }
}
