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
}
