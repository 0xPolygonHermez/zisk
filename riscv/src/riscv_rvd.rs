//! RISC-V RVD

use std::collections::HashMap;

/// RVD operation, including a map to store nested operations, if any
/// It contains a human-readable string name of the operation
pub struct RvdOperation {
    pub s: String,
    pub map: HashMap<u32, RvdOperation>,
}

/// RVD info, containing a type and an RVD operation
pub struct RvdInfo {
    pub t: String,
    pub op: RvdOperation,
}

/// RVD structure, containing a map of opcodes to RVD info instances
pub struct Rvd {
    pub opcodes: HashMap<u32, RvdInfo>,
}

/// Default constructor for Rvd structure
impl Default for Rvd {
    fn default() -> Self {
        Self::new()
    }
}

/// RVD implementation
impl Rvd {
    /// RVD constructor, setting opcodes to an empty map
    pub fn new() -> Rvd {
        Rvd { opcodes: HashMap::new() }
    }

    /// RVD initialization, creating a tree of opcode-to-operation pairs
    pub fn init(&mut self) {
        // Opcode 3
        {
            let mut info = RvdInfo {
                t: String::from("I"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.map.insert(0, RvdOperation { s: String::from("lb"), map: HashMap::new() });
            info.op.map.insert(1, RvdOperation { s: String::from("lh"), map: HashMap::new() });
            info.op.map.insert(2, RvdOperation { s: String::from("lw"), map: HashMap::new() });
            info.op.map.insert(3, RvdOperation { s: String::from("ld"), map: HashMap::new() });
            info.op.map.insert(4, RvdOperation { s: String::from("lbu"), map: HashMap::new() });
            info.op.map.insert(5, RvdOperation { s: String::from("lhu"), map: HashMap::new() });
            info.op.map.insert(6, RvdOperation { s: String::from("lwu"), map: HashMap::new() });
            self.opcodes.insert(3, info);
        }

        // Opcode 15
        {
            let mut info = RvdInfo {
                t: String::from("F"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.map.insert(0, RvdOperation { s: String::from("fence"), map: HashMap::new() });
            info.op.map.insert(1, RvdOperation { s: String::from("fence.i"), map: HashMap::new() });
            self.opcodes.insert(15, info);
        }

        // Opcode 19
        {
            let mut info = RvdInfo {
                t: String::from("I"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.map.insert(0, RvdOperation { s: String::from("addi"), map: HashMap::new() });
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("slli"), map: HashMap::new() });
                info.op.map.insert(1, op);
            }
            info.op.map.insert(2, RvdOperation { s: String::from("slti"), map: HashMap::new() });
            info.op.map.insert(3, RvdOperation { s: String::from("sltiu"), map: HashMap::new() });
            info.op.map.insert(4, RvdOperation { s: String::from("xori"), map: HashMap::new() });
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("srli"), map: HashMap::new() });
                op.map.insert(16, RvdOperation { s: String::from("srai"), map: HashMap::new() });
                info.op.map.insert(5, op);
            }
            info.op.map.insert(6, RvdOperation { s: String::from("ori"), map: HashMap::new() });
            info.op.map.insert(7, RvdOperation { s: String::from("andi"), map: HashMap::new() });
            self.opcodes.insert(19, info);
        }

        // Opcode 23
        {
            let mut info = RvdInfo {
                t: String::from("U"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.s = String::from("auipc");
            self.opcodes.insert(23, info);
        }

        // Opcode 27
        {
            let mut info = RvdInfo {
                t: String::from("I"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.map.insert(0, RvdOperation { s: String::from("addiw"), map: HashMap::new() });
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("slliw"), map: HashMap::new() });
                info.op.map.insert(1, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("srliw"), map: HashMap::new() });
                op.map.insert(16, RvdOperation { s: String::from("sraiw"), map: HashMap::new() });
                info.op.map.insert(5, op);
            }
            self.opcodes.insert(27, info);
        }

        // Opcode 35
        {
            let mut info = RvdInfo {
                t: String::from("S"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.map.insert(0, RvdOperation { s: String::from("sb"), map: HashMap::new() });
            info.op.map.insert(1, RvdOperation { s: String::from("sh"), map: HashMap::new() });
            info.op.map.insert(2, RvdOperation { s: String::from("sw"), map: HashMap::new() });
            info.op.map.insert(3, RvdOperation { s: String::from("sd"), map: HashMap::new() });
            self.opcodes.insert(35, info);
        }

        // Opcode 47
        {
            let mut info = RvdInfo {
                t: String::from("A"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(2, RvdOperation { s: String::from("lr.w"), map: HashMap::new() });
                op.map.insert(3, RvdOperation { s: String::from("sc.w"), map: HashMap::new() });
                op.map
                    .insert(1, RvdOperation { s: String::from("amoswap.w"), map: HashMap::new() });
                op.map.insert(0, RvdOperation { s: String::from("amoadd.w"), map: HashMap::new() });
                op.map.insert(4, RvdOperation { s: String::from("amoxor.w"), map: HashMap::new() });
                op.map
                    .insert(12, RvdOperation { s: String::from("amoand.w"), map: HashMap::new() });
                op.map.insert(8, RvdOperation { s: String::from("amoor.w"), map: HashMap::new() });
                op.map
                    .insert(16, RvdOperation { s: String::from("amomin.w"), map: HashMap::new() });
                op.map
                    .insert(20, RvdOperation { s: String::from("amomax.w"), map: HashMap::new() });
                op.map
                    .insert(24, RvdOperation { s: String::from("amominu.w"), map: HashMap::new() });
                op.map
                    .insert(28, RvdOperation { s: String::from("amomaxu.w"), map: HashMap::new() });
                info.op.map.insert(2, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(2, RvdOperation { s: String::from("lr.d"), map: HashMap::new() });
                op.map.insert(3, RvdOperation { s: String::from("sc.d"), map: HashMap::new() });
                op.map
                    .insert(1, RvdOperation { s: String::from("amoswap.d"), map: HashMap::new() });
                op.map.insert(0, RvdOperation { s: String::from("amoadd.d"), map: HashMap::new() });
                op.map.insert(4, RvdOperation { s: String::from("amoxor.d"), map: HashMap::new() });
                op.map
                    .insert(12, RvdOperation { s: String::from("amoand.d"), map: HashMap::new() });
                op.map.insert(8, RvdOperation { s: String::from("amoor.d"), map: HashMap::new() });
                op.map
                    .insert(16, RvdOperation { s: String::from("amomin.d"), map: HashMap::new() });
                op.map
                    .insert(20, RvdOperation { s: String::from("amomax.d"), map: HashMap::new() });
                op.map
                    .insert(24, RvdOperation { s: String::from("amominu.d"), map: HashMap::new() });
                op.map
                    .insert(28, RvdOperation { s: String::from("amomaxu.d"), map: HashMap::new() });
                info.op.map.insert(3, op);
            }
            self.opcodes.insert(47, info);
        }

        // Opcode 51
        {
            let mut info = RvdInfo {
                t: String::from("R"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("add"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("mul"), map: HashMap::new() });
                op.map.insert(32, RvdOperation { s: String::from("sub"), map: HashMap::new() });
                info.op.map.insert(0, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("sll"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("mulh"), map: HashMap::new() });
                info.op.map.insert(1, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("slt"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("mulhsu"), map: HashMap::new() });
                info.op.map.insert(2, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("sltu"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("mulhu"), map: HashMap::new() });
                info.op.map.insert(3, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("xor"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("div"), map: HashMap::new() });
                info.op.map.insert(4, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("srl"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("divu"), map: HashMap::new() });
                op.map.insert(32, RvdOperation { s: String::from("sra"), map: HashMap::new() });
                info.op.map.insert(5, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("or"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("rem"), map: HashMap::new() });
                info.op.map.insert(6, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("and"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("remu"), map: HashMap::new() });
                info.op.map.insert(7, op);
            }
            self.opcodes.insert(51, info);
        }

        // Opcode 55
        {
            let mut info = RvdInfo {
                t: String::from("U"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.s = String::from("lui");
            info.op.map.clear();
            self.opcodes.insert(55, info);
        }

        // Opcode 59
        {
            let mut info = RvdInfo {
                t: String::from("R"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("addw"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("mulw"), map: HashMap::new() });
                op.map.insert(32, RvdOperation { s: String::from("subw"), map: HashMap::new() });
                info.op.map.insert(0, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("sllw"), map: HashMap::new() });
                info.op.map.insert(1, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(1, RvdOperation { s: String::from("divw"), map: HashMap::new() });
                info.op.map.insert(4, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("srlw"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("divuw"), map: HashMap::new() });
                op.map.insert(32, RvdOperation { s: String::from("sraw"), map: HashMap::new() });
                info.op.map.insert(5, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(1, RvdOperation { s: String::from("remw"), map: HashMap::new() });
                info.op.map.insert(6, op);
            }
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(1, RvdOperation { s: String::from("remuw"), map: HashMap::new() });
                info.op.map.insert(7, op);
            }
            self.opcodes.insert(59, info);
        }

        // Opcode 99
        {
            let mut info = RvdInfo {
                t: String::from("B"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.map.insert(0, RvdOperation { s: String::from("beq"), map: HashMap::new() });
            info.op.map.insert(1, RvdOperation { s: String::from("bne"), map: HashMap::new() });
            info.op.map.insert(4, RvdOperation { s: String::from("blt"), map: HashMap::new() });
            info.op.map.insert(5, RvdOperation { s: String::from("bge"), map: HashMap::new() });
            info.op.map.insert(6, RvdOperation { s: String::from("bltu"), map: HashMap::new() });
            info.op.map.insert(7, RvdOperation { s: String::from("bgeu"), map: HashMap::new() });
            self.opcodes.insert(99, info);
        }

        // Opcode 103
        {
            let mut info = RvdInfo {
                t: String::from("I"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.map.insert(0, RvdOperation { s: String::from("jalr"), map: HashMap::new() });
            self.opcodes.insert(103, info);
        }

        // Opcode 111
        {
            let mut info = RvdInfo {
                t: String::from("J"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            info.op.s = String::from("jal");
            self.opcodes.insert(111, info);
        }

        // Opcode 115
        {
            let mut info = RvdInfo {
                t: String::from("C"),
                op: RvdOperation { s: String::new(), map: HashMap::new() },
            };
            {
                let mut op = RvdOperation { s: String::new(), map: HashMap::new() };
                op.map.insert(0, RvdOperation { s: String::from("ecall"), map: HashMap::new() });
                op.map.insert(1, RvdOperation { s: String::from("ebreak"), map: HashMap::new() });
                info.op.map.insert(0, op);
            }
            info.op.map.insert(1, RvdOperation { s: String::from("csrrw"), map: HashMap::new() });
            info.op.map.insert(2, RvdOperation { s: String::from("csrrs"), map: HashMap::new() });
            info.op.map.insert(3, RvdOperation { s: String::from("csrrc"), map: HashMap::new() });
            info.op.map.insert(5, RvdOperation { s: String::from("csrrwi"), map: HashMap::new() });
            info.op.map.insert(6, RvdOperation { s: String::from("csrrsi"), map: HashMap::new() });
            info.op.map.insert(7, RvdOperation { s: String::from("csrrci"), map: HashMap::new() });
            self.opcodes.insert(115, info);
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

    // Format Meaning              15 14 13 12  11 10 9 8 7 6 5 4 3 2 1 0
    // CR     Register             funct4       rd/rs1      rs2       op
    // CI     Immediate            funct3   imm rd/rs1      imm       op
    // CSS    Stack-relative Store funct3   imm             rs2       op
    // CIW    Wide Immediate       funct3   imm                 rd′   op
    // CL     Load                 funct3   imm       rs1′  imm rd′   op
    // CS     Store                funct3   imm       rs1′  imm rs2′  op
    // CB     Branch               funct3   offset    rs1′  offset    op
    // CJ     Jump                 funct3   jump target               op

    pub fn get_type_and_name(inst: u16) -> (&'static str, &'static str) {
        println!("Rvd::get_type_and_name() inst=0x{:x}", inst);
        // Return the type and name of the instruction
        match inst & 0x3 {
            // Check bits 1 and 0 = op2
            0x00 => {
                if inst == 0x0000 {
                    panic!("Rvd::get_type() invalid instruction 0x0000");
                }
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => ("CIW", "c.addi4spn"), // Mapped to addi
                    0x1 => ("CL", "c.fld"),       // Unmapped, i.e. not supported
                    0x2 => ("CL", "c.lw"),        // Mapped to lw
                    0x3 => ("CL", "c.ld"),        // Mapped to ld
                    0x4 => panic!("Rvd::get_type() reserved instruction inst=0x{:x}", inst),
                    0x5 => ("CS", "c.fsd"), // Unmapped, i.e. not supported
                    0x6 => ("CS", "c.sw"),  // Mapped to sw
                    0x7 => ("CS", "c.sd"),  // Mapped to sd
                    _ => panic!("Rvd::get_type() invalid logic inst=0x{:x}", inst),
                }
            }
            0x01 => match (inst >> 13) & 0x7 {
                // Check bits 15 to 13 = funct3
                0x0 => {
                    if inst == 0x1 {
                        ("CI", "c.nop") // Transpiled to ZisK nop (flag)
                    } else {
                        ("CI", "c.addi") // Mapped to addi
                    }
                }
                0x1 => ("CI", "c.addiw"), // Mapped to addiw
                0x2 => ("CI", "c.li"),    // Mapped to addi
                0x3 => {
                    if ((inst >> 7) & 0x1F) == 2 {
                        ("CL", "c.addi16sp") // Mapped to addi
                    } else {
                        ("CL", "c.lui") // Mapped to lui
                    }
                }
                0x4 => match (inst >> 12) & 0x1 {
                    0x0 => match (inst >> 10) & 0x3 {
                        0x0 => ("CI", "c.srli64"), // Mapped to srli
                        0x1 => ("CI", "c.srai64"), // Mapped to srai
                        0x2 => ("CI", "c.andi"),   // Mapped to andi
                        0x3 => match (inst >> 5) & 0x3 {
                            0x0 => ("CS", "c.sub"), // Mapped to sub
                            0x1 => ("CS", "c.xor"), // Mapped to xor
                            0x2 => ("CS", "c.or"),  // Mapped to or
                            0x3 => ("CS", "c.and"), // Mapped to and
                            _ => panic!("Rvd::get_type() invalid logic inst=0x{:x}", inst),
                        },
                        _ => panic!("Rvd::get_type() invalid logic inst=0x{:x}", inst),
                    },
                    0x1 => match (inst >> 10) & 0x3 {
                        0x3 => match (inst >> 5) & 0x3 {
                            0x0 => ("CS", "c.subw"), // Mapped to subw
                            0x1 => ("CS", "c.addw"), // Mapped to addw
                            0x2 | 0x3 => panic!("Rvd::get_type() reserved inst=0x{:x}", inst),
                            _ => panic!("Rvd::get_type() invalid logic inst=0x{:x}", inst),
                        },
                        _ => panic!("Rvd::get_type() invalid inst=0x{:x}", inst),
                    },
                    _ => panic!("Rvd::get_type() invalid logic inst=0x{:x}", inst),
                },
                0x5 => ("CJ", "c.j"),    // Mapped to jal
                0x6 => ("CB", "c.beqz"), // Mapped to beq
                0x7 => ("CB", "c.bnez"), // Mapped to bne
                _ => panic!("Rvd::get_type() invalid inst=0x{:x}", inst),
            },
            0x02 => {
                match (inst >> 13) & 0x7 {
                    // Check bits 15 to 13 = funct3
                    0x0 => ("CI", "c.slli64"), // Mapped to slli
                    0x1 => ("CI", "c.fldsp"),  // Unmapped, i.e. not supported
                    0x2 => ("CI", "c.lwsp"),   // Mapped to lw
                    0x3 => ("CI", "c.ldsp"),   // Mapped to ld
                    0x4 => {
                        match (inst >> 12) & 0x1 {
                            // Check bit 12
                            0x0 => {
                                match (inst >> 2) & 0x1F {
                                    // Check bits 6 to 2
                                    0x0 => ("CI", "c.jr"), // Mapped to jalr
                                    0x1 => ("CI", "c.mv"), // Mapped to add
                                    _ => panic!("Rvd::get_type() invalid inst=0x{:x}", inst),
                                }
                            }
                            0x1 => {
                                match (inst >> 2) & 0x1F {
                                    // Check bits 6 to 2
                                    0x0 => {
                                        match (inst >> 7) & 0x1F {
                                            // Check bits 11 to 7
                                            0x0 => ("CI", "c.ebreak"), // Mapped to ebreak
                                            _ => ("CI", "c.jalr"),     // Mapped to jalr
                                        }
                                    }
                                    _ => ("CI", "c.add"), // Mapped to add
                                }
                            }
                            _ => panic!("Rvd::get_type() invalid instruction inst=0x{:x}", inst),
                        }
                    }
                    0x5 => ("CSS", "c.fsdsp"), // Unmapped, i.e. not supported
                    0x6 => ("CSS", "c.swsp"),  // Mapped to sw
                    0x7 => ("CSS", "c.sdsp"),  // Mapped to sd
                    _ => panic!("Rvd::get_type() invalid logic inst=0x{:x}", inst),
                }
            }
            _ => panic!("Rvd::get_type() unknown opcode inst=0x{:x}", inst),
        }
    }
}
