//! RISC-V RVD
//! Based on a 16-bits or a 32-bits instruction, it returns the type and name of the instruction

/// RVD structure
pub struct Rvd {}

/// RVD implementation
impl Rvd {
    pub fn get_type_and_name_32_bits(inst: u32) -> (&'static str, &'static str) {
        match inst & 0x7F {
            3 => { // Opcode 3
                match (inst >> 12) & 0x7 {
                    0 => ("I", "lb"),
                    1 => ("I", "lh"),
                    2 => ("I", "lw"),
                    3 => ("I", "ld"),
                    4 => ("I", "lbu"),
                    5 => ("I", "lhu"),
                    6 => ("I", "lwu"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 3 inst=0x{inst:x}"),
                }
            },
            7 => { // Opcode 7
                match (inst >> 12) & 0x7 {
                    2 => ("R", "flw"),
                    3 => ("R", "fld"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 7 inst=0x{inst:x}"),
                }
            },
            15 => { // Opcode 15
                match (inst >> 12) & 0x7 {
                    0 => ("F", "fence"),
                    1 => ("F", "fence.i"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 15 inst=0x{inst:x}"),
                }
            },
            19 => { // Opcode 19
                match (inst >> 12) & 0x7 {
                    0 => ("I", "addi"),
                    1 => {
                        match (inst >> 26) & 0x3F {
                            0 => ("I", "slli"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 19 funct3=1 inst=0x{inst:x}"),
                        }
                    }
                    2 => ("I", "slti"),
                    3 => ("I", "sltiu"),
                    4 => ("I", "xori"),
                    5 => {
                        match (inst >> 26) & 0x3F {
                            0 => ("I", "srli"),
                            16 => ("I", "srai"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 19 funct3=5 inst=0x{inst:x}"),
                        }
                    },
                    6 => ("I", "ori"),
                    7 => ("I", "andi"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 19 inst=0x{inst:x}"),
                }
            },
            23 => { // Opcode 23
                ("U", "auipc")
            },
            27 => { // Opcode 27
                match (inst >> 12) & 0x7 {
                    0 => ("I", "addiw"),
                    1 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("I", "slliw"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 27 funct3=1 inst=0x{inst:x}"),
                        }
                    },
                    5 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("I", "srliw"),
                            32 => ("I", "sraiw"), // TODO: REVIEW (it was 16)
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 27 funct3=5 inst=0x{inst:x}"),
                        }
                    },
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 27 inst=0x{inst:x}"),
                }
            },
            35 => { // Opcode 35
                match (inst >> 12) & 0x7 {
                    0 => ("S", "sb"),
                    1 => ("S", "sh"),
                    2 => ("S", "sw"),
                    3 => ("S", "sd"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 35 inst=0x{inst:x}"),
                }
            },
            39 => // Opcode 39
            {
                match (inst >> 12) & 0x7 {
                    2 => ("S", "fsw"),
                    3 => ("S", "fsd"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 39 inst=0x{inst:x}"),
                }
            },
            47 => { // Opcode 47
                match (inst >> 12) & 0x7 {
                    2 => {
                        match (inst >> 27) & 0x1F {
                            2 => ("A", "lr.w"),
                            3 => ("A", "sc.w"),
                            1 => ("A", "amoswap.w"),
                            0 => ("A", "amoadd.w"),
                            4 => ("A", "amoxor.w"),
                            12 => ("A", "amoand.w"),
                            8 => ("A", "amoor.w"),
                            16 => ("A", "amomin.w"),
                            20 => ("A", "amomax.w"),
                            24 => ("A", "amominu.w"),
                            28 => ("A", "amomaxu.w"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct5 for opcode 47 funct3=2 inst=0x{inst:x}"),
                        }
                    },
                    3 => {
                        match (inst >> 27) & 0x1F {
                            2 => ("A", "lr.d"),
                            3 => ("A", "sc.d"),
                            1 => ("A", "amoswap.d"),
                            0 => ("A", "amoadd.d"),
                            4 => ("A", "amoxor.d"),
                            12 => ("A", "amoand.d"),
                            8 => ("A", "amoor.d"),
                            16 => ("A", "amomin.d"),
                            20 => ("A", "amomax.d"),
                            24 => ("A", "amominu.d"),
                            28 => ("A", "amomaxu.d"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct5 for opcode 47 funct3=3 inst=0x{inst:x}"),
                        }
                    },
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 47 inst=0x{inst:x}"),
                }
            }
            51 => { // Opcode 51
                match (inst >> 12) & 0x7 {
                    0 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "add"),
                            1 => ("R", "mul"),
                            32 => ("R", "sub"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 51 funct3=0 inst=0x{inst:x}"),
                        }
                    },
                    1 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "sll"),
                            1 => ("R", "mulh"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 51 funct3=1 inst=0x{inst:x}"),
                        }
                    },
                    2 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "slt"),
                            1 => ("R", "mulhsu"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 51 funct3=2 inst=0x{inst:x}"),
                        }
                    },
                    3 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "sltu"),
                            1 => ("R", "mulhu"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 51 funct3=3 inst=0x{inst:x}"),
                        }
                    },
                    4 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "xor"),
                            1 => ("R", "div"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 51 funct3=4 inst=0x{inst:x}"),
                        }
                    },
                    5 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "srl"),
                            1 => ("R", "divu"),
                            32 => ("R", "sra"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 51 funct3=5 inst=0x{inst:x}"),
                        }
                    },
                    6 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "or"),
                            1 => ("R", "rem"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 51 funct3=6 inst=0x{inst:x}"),
                        }
                    },
                    7 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "and"),
                            1 => ("R", "remu"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 51 funct3=7 inst=0x{inst:x}"),
                        }
                    },
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 51 inst=0x{inst:x}"),
                }
            },
            55 => { // Opcode 55
                ("U", "lui")
            },
            59 => { // Opcode 59
                match (inst >> 12) & 0x7 {
                    0 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "addw"),
                            1 => ("R", "mulw"),
                            32 => ("R", "subw"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 59 funct3=0 inst=0x{inst:x}"),
                        }
                    },
                    1 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "sllw"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 59 funct3=1 inst=0x{inst:x}"),
                        }
                    },
                    4 => {
                        match (inst >> 25) & 0x7F {
                            1 => ("R", "divw"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 59 funct3=4 inst=0x{inst:x}"),
                        }
                    },
                    5 => {
                        match (inst >> 25) & 0x7F {
                            0 => ("R", "srlw"),
                            1 => ("R", "divuw"),
                            32 => ("R", "sraw"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 59 funct3=5 inst=0x{inst:x}"),
                        }
                    },
                    6 => {
                        match (inst >> 25) & 0x7F {
                            1 => ("R", "remw"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 59 funct3=6 inst=0x{inst:x}"),
                        }
                    },
                    7 => {
                        match (inst >> 25) & 0x7F {
                            1 => ("R", "remuw"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 59 funct3=7 inst=0x{inst:x}"),
                        }
                    },
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 59 inst=0x{inst:x}"),
                }
            },
            67 => { // Opcode 67
                match (inst >> 25) & 0x3 {
                    0 => ("R4", "fmadd.s"),
                    1 => ("R4", "fmadd.d"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 67 inst=0x{inst:x}"),
                }
            },
            71 => { // Opcode 71
                match (inst >> 25) & 0x3 {
                    0 => ("R4", "fmsub.s"),
                    1 => ("R4", "fmsub.d"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 71 inst=0x{inst:x}"),
                }
            },
            75 => { // Opcode 75
                match (inst >> 25) & 0x3 {
                    0 => ("R4", "fnmsub.s"),
                    1 => ("R4", "fnmsub.d"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 75 inst=0x{inst:x}"),
                }
            },
            79 => { // Opcode 79
                match (inst >> 25) & 0x3 {
                    0 => ("R4", "fnmadd.s"),
                    1 => ("R4", "fnmadd.d"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 79 inst=0x{inst:x}"),
                }
            },
            83 => { // Opcode 83
                match (inst >> 25) & 0x7F {
                    0 => ("R", "fadd.s"),
                    1 => ("R", "fadd.d"),
                    4 => ("R", "fsub.s"),
                    5 => ("R", "fsub.d"),
                    8 => ("R", "fmul.s"),
                    9 => ("R", "fmul.d"),
                    12 => ("R", "fdiv.s"),
                    13 => ("R", "fdiv.d"),
                    16 => {
                        match (inst >> 12) & 0x7 {
                            0 => ("R", "fsgnj.s"),
                            1 => ("R", "fsgnjn.s"),
                            2 => ("R", "fsgnjx.s"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=16 inst=0x{inst:x}"),
                        }
                    },
                    17 => {
                        match (inst >> 12) & 0x7 {
                            0 => ("R", "fsgnj.d"),
                            1 => ("R", "fsgnjn.d"),
                            2 => ("R", "fsgnjx.d"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=17 inst=0x{inst:x}"),
                        }
                    },
                    20 => {
                        match (inst >> 12) & 0x7 {
                            0 => ("R", "fmin.s"),
                            1 => ("R", "fmax.s"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=20 inst=0x{inst:x}"),
                        }
                    },
                    21 => {
                        match (inst >> 12) & 0x7 {
                            0 => ("R", "fmin.d"),
                            1 => ("R", "fmax.d"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=21 inst=0x{inst:x}"),
                        }
                    },
                    32 => {
                        match (inst >> 20) & 0x1F {
                            1 => ("R", "fcvt.s.d"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=32 inst=0x{inst:x}"),
                        }
                    }
                    33 => {
                        match (inst >> 20) & 0x1F {
                            0 => ("R", "fcvt.d.s"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=33 inst=0x{inst:x}"),
                        }
                    }
                    44 => {
                        match (inst >> 20) & 0x1F {
                            0 => ("R", "fsqrt.s"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=44 inst=0x{inst:x}"),
                        }
                    },
                    45 => {
                        match (inst >> 20) & 0x1F {
                            0 => ("R", "fsqrt.d"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=45 inst=0x{inst:x}"),
                        }
                    },
                    80 => {
                        match (inst >> 12) & 0x7 {
                            2 => ("R", "feq.s"),
                            1 => ("R", "flt.s"),
                            0 => ("R", "fle.s"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=80 inst=0x{inst:x}"),
                        }
                    }
                    81 => {
                        match (inst >> 12) & 0x7 {
                            2 => ("R", "feq.d"),
                            1 => ("R", "flt.d"),
                            0 => ("R", "fle.d"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=81 inst=0x{inst:x}"),
                        }
                    }
                    96 => {
                        match (inst >> 20) & 0x1F {
                            0 => ("R", "fcvt.w.s"),
                            1 => ("R", "fcvt.wu.s"),
                            2 => ("R", "fcvt.l.s"),
                            3 => ("R", "fcvt.lu.s"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=96 inst=0x{inst:x}"),
                        }
                    }
                    97 => {
                        match (inst >> 20) & 0x1F {
                            0 => ("R", "fcvt.w.d"),
                            1 => ("R", "fcvt.wu.d"),
                            2 => ("R", "fcvt.l.d"),
                            3 => ("R", "fcvt.lu.d"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=97 inst=0x{inst:x}"),
                        }
                    }
                    104 => {
                        match (inst >> 20) & 0x1F {
                            0 => ("R", "fcvt.s.w"),
                            1 => ("R", "fcvt.s.wu"),
                            2 => ("R", "fcvt.s.l"),
                            3 => ("R", "fcvt.s.lu"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=104 inst=0x{inst:x}"),
                        }
                    }
                    105 => {
                        match (inst >> 20) & 0x1F {
                            0 => ("R", "fcvt.d.w"),
                            1 => ("R", "fcvt.d.wu"),
                            2 => ("R", "fcvt.d.l"),
                            3 => ("R", "fcvt.d.lu"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=105 inst=0x{inst:x}"),
                        }
                    }
                    112 => {
                        match (inst >> 12) & 0x7 {
                            0 => {
                                match (inst >> 20) & 0x1F {
                                    0 => ("R", "fmv.x.w"),
                                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                }
                            }
                            1 => {
                                match (inst >> 20) & 0x1F {
                                    0 => ("R", "fclass.s"),
                                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                }
                            }
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=112 inst=0x{inst:x}"),
                        }
                    }
                    113 => {
                        match (inst >> 12) & 0x7 {
                            0 => {
                                match (inst >> 20) & 0x1F {
                                    0 => ("R", "fmv.x.d"),
                                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=112 funct3=0 inst=0x{inst:x}"),
                                }
                            }
                            1 => {
                                match (inst >> 20) & 0x1F {
                                    0 => ("R", "fclass.d"),
                                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=113 funct3=0 inst=0x{inst:x}"),
                                }
                            }
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=112 inst=0x{inst:x}"),
                        }
                    }
                    120 => {
                        match (inst >> 12) & 0x7 {
                            0 => {
                                match (inst >> 20) & 0x1F {
                                    0 => ("I", "fmv.w.x"),
                                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=120 funct3=0 inst=0x{inst:x}"),
                                }
                            }
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=120 inst=0x{inst:x}"),
                        }
                    }
                    121 => {
                        match (inst >> 12) & 0x7 {
                            0 => {
                                match (inst >> 20) & 0x1F {
                                    0 => ("I", "fmv.d.x"),
                                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid rm for opcode 83 funct7=121 funct3=0 inst=0x{inst:x}"),
                                }
                            }
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 83 funct7=121 inst=0x{inst:x}"),
                        }
                    }
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct7 for opcode 83 inst=0x{inst:x}"),
                }
            },
            99 => { // Opcode 99
                match (inst >> 12) & 0x7 {
                    0 => ("B", "beq"),
                    1 => ("B", "bne"),
                    4 => ("B", "blt"),
                    5 => ("B", "bge"),
                    6 => ("B", "bltu"),
                    7 => ("B", "bgeu"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 99 inst=0x{inst:x}"),
                }
            },
            103 => { // Opcode 103
                ("I", "jalr")
            },
            111 => { // Opcode 111
                ("J", "jal")
            },
            115 => { // Opcode 115
                match (inst >> 12) & 0x7 {
                    0 => {
                        match (inst >> 20) & 0xFFF {
                            0 => ("I", "ecall"),
                            1 => ("I", "ebreak"),
                            _ => panic!("Rvd::get_type_and_name_32_bits() invalid imm for opcode 115 funct3=0 inst=0x{inst:x}"),
                        }
                    },
                    1 => ("I", "csrrw"),
                    2 => ("I", "csrrs"),
                    3 => ("I", "csrrc"),
                    5 => ("I", "csrrwi"),
                    6 => ("I", "csrrsi"),
                    7 => ("I", "csrrci"),
                    _ => panic!("Rvd::get_type_and_name_32_bits() invalid funct3 for opcode 115 inst=0x{inst:x}"),
                }
            },
            _ => panic!("Rvd::get_type_and_name_32_bits() unknown opcode inst=0x{inst:x}"),
        }
    }

    // Converta a compressed register index (e.g. rs1') to a full register index (e.g. rs1)
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
        //println!("Rvd::get_type_and_name() inst=0x{:x}", inst);
        // Return the type and name of the instruction
        match inst & 0x3 {
            // Check bits 1 and 0 = op2
            0x00 => {
                if inst == 0x0000 {
                    panic!("Rvd::get_type_and_name() invalid instruction 0x0000");
                }
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => ("CIW", "c.addi4spn"), // Mapped to addi: addi rd′, x2, nzuimm[9:2]
                    0x1 => ("CL", "c.fld"),       // Unmapped, i.e. not supported
                    0x2 => ("CL", "c.lw"),        // Mapped to lw: lw rd′, offset(rs1′)
                    0x3 => ("CL", "c.ld"),        // Mapped to ld: ld rd′, offset(rs1′)
                    0x4 => {
                        panic!("Rvd::get_type_and_name() reserved instruction inst=0x{inst:x}")
                    }
                    0x5 => ("CS", "c.fsd"), // Unmapped, i.e. not supported
                    0x6 => ("CS", "c.sw"),  // Mapped to sw: sw rs2′,offset(rs1′)
                    0x7 => ("CS", "c.sd"),  // Mapped to sd: sd rs2′, offset(rs1′)
                    _ => panic!("Rvd::get_type_and_name() invalid logic inst=0x{inst:x}"),
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
                    0x1 => ("CI", "c.srai"), // Mapped to srai: srai rd′, rd′, shamt
                    0x2 => ("CB", "c.andi"), // Mapped to andi: andi rd′, rd′, imm
                    0x3 => match (inst >> 12) & 0x1 {
                        0x0 => match (inst >> 5) & 0x3 {
                            0x0 => ("CA", "c.sub"), // Mapped to sub: sub rd′, rd′, rs2′
                            0x1 => ("CA", "c.xor"), // Mapped to xor: xor rd′, rd′, rs2′
                            0x2 => ("CA", "c.or"),  // Mapped to or: or rd′, rd′, rs2′
                            0x3 => ("CA", "c.and"), // Mapped to and: and rd′, rd′, rs2′
                            _ => panic!("Rvd::get_type_and_name() invalid logic inst=0x{inst:x}"),
                        },
                        0x01 => match (inst >> 5) & 0x3 {
                            0x0 => ("CA", "c.subw"), // Mapped to subw: subw rd′, rd′, rs2′
                            0x1 => ("CA", "c.addw"), // Mapped to addw: addw rd′, rd′,rs2′
                            0x2 | 0x3 => {
                                panic!("Rvd::get_type_and_name() reserved inst=0x{inst:x}");
                            }
                            _ => panic!("Rvd::get_type_and_name() invalid logic inst=0x{inst:x}"),
                        },
                        _ => panic!("Rvd::get_type_and_name() invalid logic inst=0x{inst:x}"),
                    },
                    _ => panic!("Rvd::get_type_and_name() invalid logic inst=0x{inst:x}"),
                },
                0x5 => ("CJ", "c.j"),    // Mapped to jal: jal x0, offset
                0x6 => ("CB", "c.beqz"), // Mapped to beq: beq rs1′, x0, offset
                0x7 => ("CB", "c.bnez"), // Mapped to bne: bne rs1′, x0, offset
                _ => panic!("Rvd::get_type_and_name() invalid inst=0x{inst:x}"),
            },
            0x02 => {
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => ("CI", "c.slli"), // Mapped to slli: slli rd, rd, shamt[5:0]
                    0x1 => ("CI", "c.fldsp"), // Unmapped, i.e. not supported
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
                            _ => panic!(
                                "Rvd::get_type_and_name() invalid instruction inst=0x{:x}",
                                inst
                            ),
                        }
                    }
                    0x5 => ("CSS", "c.fsdsp"), // Unmapped, i.e. not supported
                    0x6 => ("CSS", "c.swsp"),  // Mapped to sw: sw rs2, offset(x2)
                    0x7 => ("CSS", "c.sdsp"),  // Mapped to sd: sd rs2, offset(x2)
                    _ => panic!("Rvd::get_type_and_name() invalid logic inst=0x{inst:x}"),
                }
            }
            _ => panic!("Rvd::get_type_and_name() unknown opcode inst=0x{inst:x}"),
        }
    }
}
