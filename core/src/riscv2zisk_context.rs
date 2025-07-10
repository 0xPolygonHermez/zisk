//! Provides an interface to convert a RISC-V instruction into one or more Zisk instructions using
//! instances of ZiskInstBuilder, and accumulates these instances in a hash map as a public
//! attribute.

use riscv::{riscv_interpreter, RiscvInstruction};

use crate::{
    convert_vector, ZiskInstBuilder, ZiskRom, ARCH_ID_ZISK, INPUT_ADDR, OUTPUT_ADDR, ROM_ENTRY,
    ROM_EXIT, SYS_ADDR,
};

use std::collections::HashMap;
// The CSR precompiled addresses are defined in the `ZiskOS` `ziskos/entrypoint/src` files
// because legacy versions of Rust do not support constant parameters in `asm!` macros.

const CSR_PRECOMPILED: [&str; 11] = [
    "keccak",
    "arith256",
    "arith256_mod",
    "secp256k1_add",
    "secp256k1_dbl",
    "sha256",
    "bn254_curve_add",
    "bn254_curve_dbl",
    "bn254_complex_add",
    "bn254_complex_sub",
    "bn254_complex_mul",
];
const CSR_PRECOMPILED_ADDR_START: u32 = 0x800;
const CSR_PRECOMPILED_ADDR_END: u32 = CSR_PRECOMPILED_ADDR_START + CSR_PRECOMPILED.len() as u32;
const CSR_FCALL_ADDR_START: u32 = 0x8C0;
const CSR_FCALL_ADDR_END: u32 = 0x8DF;
const CSR_FCALL_GET_ADDR: u32 = 0xFFE;
const CSR_FCALL_PARAM_ADDR_START: u32 = 0x8F0;
const CSR_FCALL_PARAM_ADDR_END: u32 = 0x8FF;
const CSR_FCALL_PARAM_OFFSET_TO_WORDS: [u64; 16] =
    [1, 2, 4, 8, 12, 16, 20, 24, 28, 32, 48, 64, 80, 96, 128, 256];

const CAUSE_EXIT: u64 = 93;
const CSR_ADDR: u64 = SYS_ADDR + 0x8000;
const MTVEC: u64 = CSR_ADDR + 0x305;
const M64: u64 = 0xFFFFFFFFFFFFFFFF;

/// Context to store the list of converted ZisK instructions, including their program address and a
/// map to store the instructions
pub struct Riscv2ZiskContext<'a> {
    /// Next program address to assign
    s: u64,
    /// Map of program address to ZisK instructions
    pub insts: &'a mut HashMap<u64, ZiskInstBuilder>,
}

impl Riscv2ZiskContext<'_> {
    /// Converts an input RISCV instruction into a ZisK instruction and stores it into the internal
    /// map
    pub fn convert(&mut self, riscv_instruction: &RiscvInstruction) {
        match riscv_instruction.inst.as_str() {
            "lb" => self.load_op(riscv_instruction, "signextend_b", 1),
            "lbu" => self.load_op(riscv_instruction, "copyb", 1),
            "lh" => self.load_op(riscv_instruction, "signextend_h", 2),
            "lhu" => self.load_op(riscv_instruction, "copyb", 2),
            "lw" => self.load_op(riscv_instruction, "signextend_w", 4),
            "lwu" => self.load_op(riscv_instruction, "copyb", 4),
            "ld" => self.load_op(riscv_instruction, "copyb", 8),
            "fence" => self.nop(riscv_instruction),
            "fence.i" => self.nop(riscv_instruction),
            "addi" => self.immediate_op_or_x0_copyb(riscv_instruction, "add"),
            "slli" => self.immediate_op(riscv_instruction, "sll"),
            "slti" => self.immediate_op(riscv_instruction, "lt"),
            "sltiu" => self.immediate_op(riscv_instruction, "ltu"),
            "xori" => self.immediate_op_or_x0_copyb(riscv_instruction, "xor"),
            "srli" => self.immediate_op(riscv_instruction, "srl"),
            "srai" => self.immediate_op(riscv_instruction, "sra"),
            "ori" => self.immediate_op_or_x0_copyb(riscv_instruction, "or"),
            "andi" => self.immediate_op(riscv_instruction, "and"),
            "auipc" => self.auipc(riscv_instruction),
            "addiw" => self.immediate_op(riscv_instruction, "add_w"),
            "slliw" => self.immediate_op(riscv_instruction, "sll_w"),
            "srliw" => self.immediate_op(riscv_instruction, "srl_w"),
            "sraiw" => self.immediate_op(riscv_instruction, "sra_w"),
            "sb" => self.store_op(riscv_instruction, "copyb", 1),
            "sh" => self.store_op(riscv_instruction, "copyb", 2),
            "sw" => self.store_op(riscv_instruction, "copyb", 4),
            "sd" => self.store_op(riscv_instruction, "copyb", 8),
            "lr.w" => self.load_op(riscv_instruction, "signextend_w", 4),
            "sc.w" => self.sc_w(riscv_instruction),
            "amoswap.w" => self.create_atomic_swap(riscv_instruction, "signextend_w", "copyb", 4),
            "amoadd.w" => {
                self.create_atomic_op(riscv_instruction, "signextend_w", "add_w", "copyb", 4)
            }
            "amoxor.w" => {
                self.create_atomic_op(riscv_instruction, "signextend_w", "xor", "copyb", 4)
            }
            "amoand.w" => {
                self.create_atomic_op(riscv_instruction, "signextend_w", "and", "copyb", 4)
            }
            "amoor.w" => self.create_atomic_op(riscv_instruction, "signextend_w", "or", "copyb", 4),
            "amomin.w" => {
                self.create_atomic_op(riscv_instruction, "signextend_w", "min_w", "copyb", 4)
            }
            "amomax.w" => {
                self.create_atomic_op(riscv_instruction, "signextend_w", "max_w", "copyb", 4)
            }
            "amominu.w" => {
                self.create_atomic_op(riscv_instruction, "signextend_w", "minu_w", "copyb", 4)
            }
            "amomaxu.w" => {
                self.create_atomic_op(riscv_instruction, "signextend_w", "maxu_w", "copyb", 4)
            }
            "lr.d" => self.load_op(riscv_instruction, "copyb", 8),
            "sc.d" => self.sc_d(riscv_instruction),
            "amoswap.d" => self.create_atomic_swap(riscv_instruction, "copyb", "copyb", 8),
            "amoadd.d" => self.create_atomic_op(riscv_instruction, "copyb", "add", "copyb", 8),
            "amoxor.d" => self.create_atomic_op(riscv_instruction, "copyb", "xor", "copyb", 8),
            "amoand.d" => self.create_atomic_op(riscv_instruction, "copyb", "and", "copyb", 8),
            "amoor.d" => self.create_atomic_op(riscv_instruction, "copyb", "or", "copyb", 8),
            "amomin.d" => self.create_atomic_op(riscv_instruction, "copyb", "min", "copyb", 8),
            "amomax.d" => self.create_atomic_op(riscv_instruction, "copyb", "max", "copyb", 8),
            "amominu.d" => self.create_atomic_op(riscv_instruction, "copyb", "minu", "copyb", 8),
            "amomaxu.d" => self.create_atomic_op(riscv_instruction, "copyb", "maxu", "copyb", 8),
            "add" => self.create_register_op(riscv_instruction, "add"),
            "mul" => self.create_register_op(riscv_instruction, "mul"),
            "sub" => self.create_register_op(riscv_instruction, "sub"),
            "sll" => self.create_register_op(riscv_instruction, "sll"),
            "mulh" => self.create_register_op(riscv_instruction, "mulh"),
            "slt" => self.create_register_op(riscv_instruction, "lt"),
            "mulhsu" => self.create_register_op(riscv_instruction, "mulsuh"),
            "sltu" => self.create_register_op(riscv_instruction, "ltu"),
            "mulhu" => self.create_register_op(riscv_instruction, "muluh"),
            "xor" => self.create_register_op(riscv_instruction, "xor"),
            "div" => self.create_register_op(riscv_instruction, "div"),
            "srl" => self.create_register_op(riscv_instruction, "srl"),
            "divu" => self.create_register_op(riscv_instruction, "divu"),
            "sra" => self.create_register_op(riscv_instruction, "sra"),
            "or" => self.create_register_op(riscv_instruction, "or"),
            "rem" => self.create_register_op(riscv_instruction, "rem"),
            "and" => self.create_register_op(riscv_instruction, "and"),
            "remu" => self.create_register_op(riscv_instruction, "remu"),
            "lui" => self.lui(riscv_instruction),
            "addw" => self.create_register_op(riscv_instruction, "add_w"),
            "mulw" => self.create_register_op(riscv_instruction, "mul_w"),
            "subw" => self.create_register_op(riscv_instruction, "sub_w"),
            "sllw" => self.create_register_op(riscv_instruction, "sll_w"),
            "divw" => self.create_register_op(riscv_instruction, "div_w"),
            "srlw" => self.create_register_op(riscv_instruction, "srl_w"),
            "divuw" => self.create_register_op(riscv_instruction, "divu_w"),
            "sraw" => self.create_register_op(riscv_instruction, "sra_w"),
            "remw" => self.create_register_op(riscv_instruction, "rem_w"),
            "remuw" => self.create_register_op(riscv_instruction, "remu_w"),
            "beq" => self.create_branch_op(riscv_instruction, "eq", false),
            "bne" => self.create_branch_op(riscv_instruction, "eq", true),
            "blt" => self.create_branch_op(riscv_instruction, "lt", false),
            "bge" => self.create_branch_op(riscv_instruction, "lt", true),
            "bltu" => self.create_branch_op(riscv_instruction, "ltu", false),
            "bgeu" => self.create_branch_op(riscv_instruction, "ltu", true),
            "jalr" => self.jalr(riscv_instruction),
            "jal" => self.jal(riscv_instruction),
            "ecall" => self.ecall(riscv_instruction),
            "ebreak" => self.nop(riscv_instruction),
            "csrrw" => self.csrrw(riscv_instruction),
            "csrrs" => self.csrrs(riscv_instruction),
            "csrrc" => self.csrrc(riscv_instruction),
            "csrrwi" => self.csrrwi(riscv_instruction),
            "csrrsi" => self.csrrsi(riscv_instruction),
            "csrrci" => self.csrrci(riscv_instruction),
            _ => panic!(
                "Riscv2ZiskContext::convert() found invalid riscv_instruction.inst={}",
                riscv_instruction.inst
            ),
        }
    }

    /*amoadd.w rs1, rs2, rd
    if rd != rs2 != rs1
        signextend_w([%rs1], [a]) -> [%rd], j(pc+1, pc+1)
        add_w(last_c, [%rs2]), j(pc+1, pc+1)
        copyb_w( [%rs1] , last_c) -> [a], j(pc+2, pc+2)
    else rs1 != (rs2 == rd)
        signextend_w([%rs1], [a]) -> [%tmp1], j(pc+1, pc+1)
        add_w(last_c, [%rs2]), j(pc+1, pc+1)
        copyb_w( [%rs1] , last_c) -> [a], j(pc+1, pc+1)
        copyb_d(0, [%tmp1]) -> [%rd], j(pc+1, pc+1), j(pc+1, pc+1)*/

    /// Creates a set of Zisk operations that implement a RISC-V atomic operation,
    /// i.e. a load-modify-store operation
    pub fn create_atomic_op(
        &mut self,
        i: &RiscvInstruction,
        loadf: &str,
        op: &str,
        storef: &str,
        w: u64,
    ) {
        if (i.rd != i.rs1) && (i.rd != i.rs2) {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.ind_width(w);
                zib.src_b("ind", 0, false);
                zib.op(loadf).unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(1, 1);
                zib.verbose(&format!("{} r{}, r{}, r{}", i.inst, i.rs1, i.rs2, i.rd));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("lastc", 0, false);
                zib.src_b("reg", i.rs2 as u64, false);
                zib.op(op).unwrap();
                zib.j(1, 1);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.ind_width(w);
                zib.src_b("lastc", 0, false);
                zib.op(storef).unwrap();
                zib.store("ind", 0, false, false);
                zib.j(2, 2);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 2;
            }
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.ind_width(w);
                zib.src_b("ind", 0, false);
                zib.op(loadf).unwrap();
                zib.store("reg", 32, false, false);
                zib.j(1, 1);
                zib.verbose(&format!("{} r{}, r{}, r{}", i.inst, i.rs1, i.rs2, i.rd));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("lastc", 0, false);
                zib.src_b("reg", i.rs2 as u64, false);
                zib.op(op).unwrap();
                zib.j(1, 1);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.ind_width(w);
                zib.src_b("lastc", 0, false);
                zib.op(storef).unwrap();
                zib.store("ind", 0, false, false);
                zib.j(1, 1);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("reg", 32, false);
                zib.op("copyb").unwrap();
                zib.j(1, 1);
                zib.store("reg", i.rd as i64, false, false);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
        }
    }

    //amoswap.w rs1, rs2, rd
    //if rd != rs2
    //    signextend_w([%rs1], [a]) -> [%rd], j(pc+1, pc+1)
    //    copyb_w( same_a , [rs2]) -> [a], j(pc+3, pc+3)
    //else
    //    signextend_w([%rs1], [a]) -> [%tmp1], j(pc+1, pc+1)
    //    copyb_w( same_a , [rs2]) -> [a], j(pc+1, pc+1)
    //    copyb_d(0, [%tmp1]) -> [%rd], j(pc+2, pc+2)

    /// Creates a set of Zisk operations that implement a RISC-V atomic swap operation
    pub fn create_atomic_swap(&mut self, i: &RiscvInstruction, loadf: &str, storef: &str, w: u64) {
        if (i.rd != i.rs1) && (i.rd != i.rs2) {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.ind_width(w);
                zib.src_b("ind", 0, false);
                zib.op(loadf).unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(1, 1);
                zib.verbose(&format!("{} r{}, r{}, r{}", i.inst, i.rs1, i.rs2, i.rd));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("reg", i.rs2 as u64, false);
                zib.op(storef).unwrap();
                zib.ind_width(w);
                zib.store("ind", 0, false, false);
                zib.j(3, 3);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 3;
            }
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.ind_width(w);
                zib.src_b("ind", 0, false);
                zib.op(loadf).unwrap();
                zib.store("reg", 32, false, false);
                zib.j(1, 1);
                zib.verbose(&format!("{} r{}, r{}, r{}", i.inst, i.rs1, i.rs2, i.rd));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("reg", i.rs2 as u64, false);
                zib.op(storef).unwrap();
                zib.ind_width(w);
                zib.store("ind", 0, false, false);
                zib.j(1, 1);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("reg", 32, false);
                zib.op("copyb").unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(2, 2);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 2;
            }
        }
    }

    /// Creates a Zisk operation that implements a RISC-V register operation, i.e. an operation that
    /// loads both input parameters a and b from their respective registers,
    /// and stores the result c into a register
    pub fn create_register_op(&mut self, i: &RiscvInstruction, op: &str) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("reg", i.rs1 as u64, false);
        zib.src_b("reg", i.rs2 as u64, false);
        zib.op(op).unwrap();
        zib.store("reg", i.rd as i64, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("{} r{}, r{}, r{}", i.inst, i.rd, i.rs1, i.rs2));
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    // beq rs1, rs2, label
    //    eq([%rs1], [rs2]), j(label)

    /// Creates a Zisk operation that implements a RISC-V branch operation, i.e. an operation that
    /// jumps to another operation, or continues the normal execution, based on a condition
    /// specifies by the operation
    pub fn create_branch_op(&mut self, i: &RiscvInstruction, op: &str, neg: bool) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("reg", i.rs1 as u64, false);
        zib.src_b("reg", i.rs2 as u64, false);
        zib.verbose(&format!("{} r{}, r{}, 0x{:x}", i.inst, i.rs1, i.rs2, i.imm));
        zib.op(op).unwrap();
        if neg {
            zib.j(4, i.imm);
        } else {
            zib.j(i.imm, 4);
        }
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    /// Creates a Zisk flag operation that simply sets the flag to true and continues the execution
    /// to the next operation
    pub fn nop(&mut self, i: &RiscvInstruction) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", 0, false);
        zib.op("flag").unwrap();
        zib.j(4, 4);
        zib.verbose(&i.inst.to_string());
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    // lb rd, imm(rs1)
    //    signextend_b([%rs1], [a + imm]) -> [%rd]

    /// Creates a Zisk operation that loads a value from memory using the specified operation
    /// and stores the result in a register
    pub fn load_op(&mut self, i: &RiscvInstruction, op: &str, w: u64) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("reg", i.rs1 as u64, false);
        zib.ind_width(w);
        zib.src_b("ind", i.imm as u64, false);
        zib.op(op).unwrap();
        zib.store("reg", i.rd as i64, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("{} r{}, 0x{:x}(r{})", i.inst, i.rd, i.imm, i.rs1));
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    // sb rs2, imm(rs1)
    //    copyb_d([%rs1], [%rs2]) -> [a + imm]

    /// Creates a Zisk operation that loads a value from register using the specified operation
    /// and stores the result in memory
    pub fn store_op(&mut self, i: &RiscvInstruction, op: &str, w: u64) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("reg", i.rs1 as u64, false);
        zib.src_b("reg", i.rs2 as u64, false);
        zib.op(op).unwrap();
        zib.ind_width(w);
        zib.store("ind", i.imm as i64, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("{} r{}, 0x{}(r{})", i.inst, i.rs2, i.imm, i.rs1));
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    // addi rd, rs1, imm
    //      add([%rs1], imm) -> [%rd]

    /// Creates a Zisk operation that loads a constant value using the specified operation and
    /// stores the result in a register
    pub fn immediate_op(&mut self, i: &RiscvInstruction, op: &str) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("reg", i.rs1 as u64, false);
        zib.src_b("imm", i.imm as u64, false);
        zib.op(op).unwrap();
        zib.store("reg", i.rd as i64, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("{} r{}, r{}, 0x{:x}", i.inst, i.rd, i.rs1, i.imm));
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    // addi rd, rs1, imm
    //      add([%rs1], imm) -> [%rd]

    /// Creates a Zisk operation that loads a constant value using the specified operation and
    /// stores the result in a register, if rs1 is x0, operation is replaced by copyb, only could
    /// be use on operations that op(x0, imm) == imm (e.g. add, or, xor)
    pub fn immediate_op_or_x0_copyb(&mut self, i: &RiscvInstruction, op: &str) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("reg", i.rs1 as u64, false);
        zib.src_b("imm", i.imm as u64, false);
        if i.rs1 == 0 {
            zib.op("copyb").unwrap();
            zib.verbose(&format!("{} r{}, r{}, 0x{:x} => copyb", i.inst, i.rd, i.rs1, i.imm));
        } else {
            zib.op(op).unwrap();
            zib.verbose(&format!("{} r{}, r{}, 0x{:x}", i.inst, i.rd, i.rs1, i.imm));
        }
        zib.store("reg", i.rd as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    // auipc rd, upimm
    //     flag(0,0), j(pc+upimm<<12, pc+4) -> [%rd]    // 4 goes to jmp_offset2 and upimm << 12 to
    // jmp_offset1
    pub fn auipc(&mut self, i: &RiscvInstruction) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", 0, false);
        zib.op("flag").unwrap();
        zib.store_ra("reg", i.rd as i64, false);
        zib.j(4, i.imm);
        zib.verbose(&format!("auipc r{}, 0x{:x}", i.rd, i.imm));
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    // sc.w rd, rs2, (rs1)
    //    copyb_d([%rs1], [%rs2]) -> [a]
    //    copyb_d(0,0) -> [%rd]
    /// Implements the RISC-V store-conditional instruction of a 32-bits value
    pub fn sc_w(&mut self, i: &RiscvInstruction) {
        if i.rd > 0 {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("reg", i.rs2 as u64, false);
                zib.op("copyb").unwrap();
                zib.ind_width(4);
                zib.store("ind", 0, false, false);
                zib.j(1, 1);
                zib.verbose(&format!("sc.w r{}, r{}, (r{})", i.rd, i.rs2, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("imm", 0, false);
                zib.op("copyb").unwrap();
                zib.ind_width(4);
                zib.store("reg", i.rd as i64, false, false);
                zib.j(3, 3);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 3;
            }
        } else {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("copyb").unwrap();
            zib.ind_width(4);
            zib.store("ind", 0, false, false);
            zib.j(4, 4);
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        }
    }

    // sc.d rd, rs2, (rs1)
    //    copyb([%rs1], [%rs2]) -> [a]
    //    copyb(0,0) -> [%rd]
    /// Implements the RISC-V store-conditional instruction of a 64-bits value
    pub fn sc_d(&mut self, i: &RiscvInstruction) {
        if i.rd > 0 {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("reg", i.rs2 as u64, false);
                zib.op("copyb").unwrap();
                zib.ind_width(8);
                zib.store("ind", 0, false, false);
                zib.j(1, 1);
                zib.verbose(&format!("sc.w r{}, r{}, (r{})", i.rd, i.rs2, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("imm", 0, false);
                zib.op("copyb").unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(3, 3);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 3;
            }
        } else {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("copyb").unwrap();
            zib.ind_width(8);
            zib.store("ind", 0, false, false);
            zib.j(4, 4);
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        }
    }

    // lui rd, imm
    //      copyb_b(0, imm) -> [rd]
    /// Implementes the RISC-V load-upper-immediate instruction to load a 32-bits constant
    pub fn lui(&mut self, i: &RiscvInstruction) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", i.imm as u64, false);
        zib.op("copyb").unwrap();
        zib.store("reg", i.rd as i64, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("lui r{}, 0x{:x}", i.rd, i.imm));
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    //     jalr rd, rs1, imm
    //          copyb_d(0, [%rs1]), j(c + imm) -> [rd]
    /// Implements the RISC-V jump-and-link-register inconditional jump instruction
    pub fn jalr(&mut self, i: &RiscvInstruction) {
        if (i.imm % 4) == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("imm", 0xfffffffffffffffc, false);
            zib.src_b("reg", i.rs1 as u64, false);
            zib.op("and").unwrap();
            zib.set_pc();
            zib.store_ra("reg", i.rd as i64, false);
            zib.j(i.imm, 4);
            zib.verbose(&format!("jalr r{}, r{}, 0x{:x}", i.rd, i.rs1, i.imm));
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", i.imm as u64, false);
                zib.src_b("reg", i.rs1 as u64, false);
                zib.op("add").unwrap();
                zib.j(1, 1);
                zib.verbose(&format!("jalr r{}, r{}, 0x{:x} ; 1/2", i.rd, i.rs1, i.imm));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 1;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0xfffffffffffffffc, false);
                zib.src_b("lastc", 0, false);
                zib.op("and").unwrap();
                zib.set_pc();
                zib.store_ra("reg", i.rd as i64, false);
                zib.j(0, 3);
                zib.verbose(&format!("jalr r{}, r{}, 0x{:x} ; 2/2", i.rd, i.rs1, i.imm));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 3;
            }
        }
    }

    //    jal rd, label
    //          flag(0,0), j(pc + imm) -> [rd]
    /// Implements the RISC-V jump-and-link inconditional jump instruction
    pub fn jal(&mut self, i: &RiscvInstruction) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", 0, false);
        zib.op("flag").unwrap();
        zib.store_ra("reg", i.rd as i64, false);
        zib.j(i.imm, 4);
        zib.verbose(&format!("jal r{}, 0x{:x}", i.rd, i.imm));
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    /// Makes a system call
    pub fn ecall(&mut self, _i: &RiscvInstruction) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("imm", 0, false);
        zib.src_b("mem", MTVEC, false);
        zib.op("copyb").unwrap();
        zib.store_ra("reg", 1, false);
        zib.set_pc();
        zib.j(0, 4);
        zib.verbose("ecall");
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

    // RISC-V defines a separate address space of 4096 Control and Status registers associated with
    // each hart. All CSR instructions atomically read-modify-write a single CSR,

    /*
    csrrw rd, csr, rs1
        if (rd == rs1) {
            if (rd == 0) {
                copyb(0, 0) -> [%csr]
            } else {
                copyb(0, [csr]) -> [%t0]
                copyb(0, [%rs1]) -> [csr]
                copyb(0, [%t0]) -> [%rd]
            }
        } else {
            if (rd == 0) {
                copyb(0, [%rs1]) -> [csr]
            } else {
                copyb(0, [csr]) -> [%rd]
                copyb(0, [%rs1]) -> [csr]
            }
        }
    */

    /// The CSRRW (Atomic Read/Write CSR) instruction atomically swaps values in the CSRs and
    /// integer registers. CSRRW reads the old value of the CSR, zero-extends the value to XLEN
    /// bits, then writes it to integer register rd. The initial value in rs1 is written to the CSR.
    /// If rd=x0, then the instruction shall not read the CSR and shall not cause any of the side
    /// effects that might occur on a CSR read.
    pub fn csrrw(&mut self, i: &RiscvInstruction) {
        if i.rd == i.rs1 {
            if i.rd == 0 {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("imm", 0, false);
                zib.op("copyb").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(4, 4);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} #rd=rs1=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            } else {
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("imm", 0, false);
                    zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                    zib.op("copyb").unwrap();
                    zib.store("reg", 33, false, false);
                    zib.j(1, 1);
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("imm", 0, false);
                    zib.src_b("reg", i.rs1 as u64, false);
                    zib.op("copyb").unwrap();
                    zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                    zib.j(1, 1);
                    zib.verbose(&format!(
                        "{} r{}, 0x{:x}, r{} #rd=rs1!=0",
                        i.inst, i.rd, i.csr, i.rs1
                    ));
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("imm", 0, false);
                    zib.src_b("reg", 33, false);
                    zib.op("copyb").unwrap();
                    zib.store("reg", i.rd as i64, false, false);
                    zib.j(2, 2);
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
            }
        } else if i.rd == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("imm", 0, false);
            zib.src_b("reg", i.rs1 as u64, false);
            zib.op("copyb").unwrap();
            zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, 0x{:x}, r{} #rs1!=rd=0", i.inst, i.rd, i.csr, i.rs1));
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                zib.op("copyb").unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(1, 1);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, r{} #rs1!=rd && rd!=0",
                    i.inst, i.rd, i.csr, i.rs1
                ));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("reg", i.rs1 as u64, false);
                zib.op("copyb").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(3, 3);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        }
    }

    /*
    csrrs rd, csr, rs1
        if (rd == rs1) {
            if (rd == 0) {
                copyb(0, 0) /NOP
            } else {
                copyb(0, [csr]) -> [%t0]
                or([%t0], [%rs1]) -> [csr]
                copyb(0, [%t0]) -> [%rd]
            }
        } else {
            if (rd == 0) {
                or([csr], [%rs1]) -> [csr]
            } else if (rs1 == 0)
                copyb(0, [csr]) -> [rd]
            } else {
                copyb(0, [csr]) -> [%rd]
                or([%rd], [%rs1]) -> [csr]
            }
        }
    */

    /// The CSRRS (Atomic Read and Set Bits in CSR) instruction reads the value of the CSR,
    /// zero-extends the value to XLEN bits, and writes it to integer register rd. The initial value
    /// in integer register rs1 is treated as a bit mask that specifies bit positions to be set in
    /// the CSR. Any bit that is high in rs1 will cause the corresponding bit to be set in the CSR,
    /// if that CSR bit is writable.
    pub fn csrrs(&mut self, i: &RiscvInstruction) {
        if i.rd == i.rs1 {
            if i.rd == 0 {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.op("copyb").unwrap();
                zib.src_a("imm", 0, false);
                zib.src_b("imm", 0, false);
                zib.j(4, 4);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} ## rd=rs=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            } else {
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("imm", 0, false);
                    zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                    zib.op("copyb").unwrap();
                    zib.store("reg", 33, false, false);
                    zib.j(1, 1);
                    zib.verbose(&format!(
                        "{} r{}, 0x{:x}, r{} # rd=rs!=0",
                        i.inst, i.rd, i.csr, i.rs1
                    ));
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("lastc", 0, false);
                    zib.src_b("reg", i.rs1 as u64, false);
                    zib.op("or").unwrap();
                    zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                    zib.j(1, 1);
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("imm", 0, false);
                    zib.src_b("reg", 33, false);
                    zib.op("copyb").unwrap();
                    zib.store("reg", i.rd as i64, false, false);
                    zib.j(2, 2);
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
            }
        } else if i.rd == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_b("reg", i.rs1 as u64, false);
            zib.j(4, 4);
            if (CSR_PRECOMPILED_ADDR_START..=CSR_PRECOMPILED_ADDR_END).contains(&i.csr) {
                zib.src_a("step", 0, false);
                let precompiled = CSR_PRECOMPILED[(i.csr - CSR_PRECOMPILED_ADDR_START) as usize];
                zib.op(precompiled).unwrap();
                zib.verbose(precompiled);
            } else if (CSR_FCALL_PARAM_ADDR_START..=CSR_FCALL_PARAM_ADDR_END).contains(&i.csr) {
                let words =
                    CSR_FCALL_PARAM_OFFSET_TO_WORDS[(i.csr - CSR_FCALL_PARAM_ADDR_START) as usize];
                zib.src_a("imm", words, false);
                zib.op("fcall_param").unwrap();
                zib.verbose(&format!(
                    "csrrs 0x{0:X}, rs1={1} => copyb[fcall_param(r{1},{2})]",
                    i.csr, i.rs1, words
                ));
            } else {
                zib.src_a("mem", CSR_ADDR + i.csr as u64, false);
                zib.op("or").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} # rs!=rd=0", i.inst, i.rd, i.csr, i.rs1));
            }
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        } else if i.rs1 == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("imm", 0, false);
            if i.csr == CSR_FCALL_GET_ADDR {
                zib.src_b("mem", INPUT_ADDR, false);
                zib.op("fcall_get").unwrap();
                zib.verbose(&format!(
                    "csrrs rd={}, 0x{:X}, rs1={} => copyb[fcall_get]",
                    i.rd, i.csr, i.rs1
                ));
            } else {
                zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                zib.op("copyb").unwrap();
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} #rd!=rs=0", i.inst, i.rd, i.csr, i.rs1));
            }
            zib.store("reg", i.rd as i64, false, false);
            zib.j(4, 4);
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                zib.op("copyb").unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(1, 1);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} #rd!=rs!=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("lastc", 0, false);
                zib.src_b("reg", i.rs1 as u64, false);
                zib.op("or").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(3, 3);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        }
    }

    /*
    csrrc rd, csr, rs1
        if (rd == rs1) {
            if (rd == 0) {
                copyb(0, 0) /NOP
            } else {
                copyb(0, [csr]) -> [%t0]
                xor(MASK, [%rs1])
                and([%t0], lastc) -> [csr]
                copyb(0, [%t0]) -> [%rd]
            }
        } else {
            if (rd == 0) {
                xor(MASK, [%rs1])
                and([csr], lastc) -> [csr]
            } else if (rs1 == 0)
                copyb(0, [csr]) -> [rd]
            } else {
                copyb(0, [csr]) -> [%rd]
                xor(MASK, [%rs1])
                and([%rd], lastc) -> [csr]
            }
        }
    */

    /// The CSRRC (Atomic Read and Clear Bits in CSR) instruction reads the value of the CSR,
    /// zero-extends the value to XLEN bits, and writes it to integer register rd. The initial value
    /// in integer register rs1 is treated as a bit mask that specifies bit positions to be cleared
    /// in the CSR. Any bit that is high in rs1 will cause the corresponding bit to be cleared in
    /// the CSR, if that CSR bit is writable.
    pub fn csrrc(&mut self, i: &RiscvInstruction) {
        if i.rd == i.rs1 {
            if i.rd == 0 {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("imm", 0, false);
                zib.op("copyb").unwrap();
                zib.j(4, 4);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} ## rd=rs=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            } else {
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("imm", 0, false);
                    zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                    zib.op("copyb").unwrap();
                    zib.store("reg", 33, false, false);
                    zib.j(1, 1);
                    zib.verbose(&format!(
                        "{} r{}, 0x{:x}, r{} # rd=rs!=0",
                        i.inst, i.rd, i.csr, i.rs1
                    ));
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("imm", M64, false);
                    zib.src_b("reg", i.rs1 as u64, false);
                    zib.op("xor").unwrap();
                    zib.j(1, 1);
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("reg", 33, false);
                    zib.src_b("lastc", 0, false);
                    zib.op("and").unwrap();
                    zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                    zib.j(1, 1);
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("mem", 0, false);
                    zib.src_b("reg", 33, false);
                    zib.op("copyb").unwrap();
                    zib.store("reg", i.rd as i64, false, false);
                    zib.j(1, 1);
                    zib.build();
                    self.insts.insert(self.s, zib);
                    self.s += 4;
                }
            }
        } else if i.rd == 0 {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", M64, false);
                zib.src_b("reg", i.rs1 as u64, false);
                zib.op("xor").unwrap();
                zib.j(1, 1);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} # rs!=rd=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", CSR_ADDR + i.csr as u64, false);
                zib.src_b("lastc", 0, false);
                zib.op("and").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(3, 3);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} # rs!=rd=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        } else if i.rs1 == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("imm", 0, false);
            zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
            zib.op("copyb").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, 0x{:x}, r{} #rd!=rs=0", i.inst, i.rd, i.csr, i.rs1));
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", 0, false);
                zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                zib.op("copyb").unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(1, 1);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} #rd!=rs!=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", M64, false);
                zib.src_b("reg", i.rs1 as u64, false);
                zib.op("xor").unwrap();
                zib.j(1, 1);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("reg", i.rd as u64, false);
                zib.src_b("lastc", 0, false);
                zib.op("and").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(2, 2);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        }
    }

    /*
    csrrci rd, csr
        {
            if (rd == 0) {
                copyb(0, imme) -> [csr]
            } else {
                copyb(0, [csr]) -> [%rd]
                copyb(0, imme) -> [csr]
            }
        }
    */
    /// The CSRRWI, CSRRSI, and CSRRCI variants are similar to CSRRW, CSRRS, and CSRRC respectively,
    /// except they update the CSR using an XLEN-bit value obtained by zero-extending a 5-bit
    /// unsigned immediate (`uimm[4:0]`) field encoded in the rs1 field instead of a value from an
    /// integer register.
    pub fn csrrwi(&mut self, i: &RiscvInstruction) {
        if i.rd == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);

            if (CSR_FCALL_ADDR_START..=CSR_FCALL_ADDR_END).contains(&i.csr) {
                let func_id = (((i.csr - CSR_FCALL_ADDR_START) as u64) << 5) + i.imme as u64;
                zib.src_a("imm", func_id, false);
                zib.src_b("imm", 0, false);
                zib.op("fcall").unwrap();
                zib.verbose(&format!(
                    "csrrs 0x{:X}, imm={} => copyb[fcall({})]",
                    i.csr, i.rs1, func_id
                ));
                // anything to store
            } else {
                zib.src_a("imm", 0, false);
                zib.src_b("imm", i.imme as u64, false);
                zib.op("copyb").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, 0x{:x} #rd = 0",
                    i.inst, i.rd, i.csr, i.imme
                ));
            }
            zib.j(4, 4);
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", 0, false);
                zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                zib.op("copyb").unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(1, 1);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, 0x{:x} #rd != 0",
                    i.inst, i.rd, i.csr, i.imme
                ));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", 0, false);
                zib.src_b("imm", i.imme as u64, false);
                zib.op("copyb").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(3, 3);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        }
    }

    /*
    csrrsi rd, csr, rs1
        if (rd == 0) {
            if (imme == 0) {
                copyb(0,0) ; nop
            } else {
                or([csr], imme) -> [csr]
            }
        } else {
            if (imme == 0) {
                copyb(0, [csr]) -> [%rd]
            } else {
                copyb(0, [csr]) -> [%rd]
                or([%rd], imme) -> [csr]
            }
        }
    */
    pub fn csrrsi(&mut self, i: &RiscvInstruction) {
        if i.rd == 0 {
            if i.imme == 0 {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("imm", 0, false);
                zib.op("copyb").unwrap();
                zib.j(4, 4);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, r{} # rd=0 imm=0",
                    i.inst, i.rd, i.csr, i.rs1
                ));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            } else {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", CSR_ADDR + i.csr as u64, false);
                zib.src_b("imm", i.imme as u64, false);
                zib.op("or").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(4, 4);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, r{} # rd=0 imm!=0",
                    i.inst, i.rd, i.csr, i.rs1
                ));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        } else if i.imme == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("imm", 0, false);
            zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
            zib.op("copyb").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, 0x{:x}, r{} # rd!=0 imm=0", i.inst, i.rd, i.csr, i.rs1));
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", 0, false);
                zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                zib.op("copyb").unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(1, 1);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, r{} # rd!=0 imm!=0",
                    i.inst, i.rd, i.csr, i.rs1
                ));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("lastc", 0, false);
                zib.src_b("imm", i.imme as u64, false);
                zib.op("or").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(3, 3);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        }
    }

    /*
    csrci rd, csr, rs1
        if (rd == 0) {
            if (imme == 0) {
                copyb(0,0) ; nop
            } else {
                and([csr], not(imme)) -> [csr]
            }
        } else {
            if (imme == 0) {
                copyb(0, [csr]) -> [%rd]
            } else {
                copyb(0, [csr]) -> [%rd]
                and([%rd], not(imme)) -> [csr]
            }
        }
    */
    pub fn csrrci(&mut self, i: &RiscvInstruction) {
        if i.rd == 0 {
            if i.imme == 0 {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("imm", 0, false);
                zib.src_b("imm", 0, false);
                zib.op("copyb").unwrap();
                zib.j(4, 4);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, r{} # rd=0 imm=0",
                    i.inst, i.rd, i.csr, i.rs1
                ));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            } else {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", CSR_ADDR + i.csr as u64, false);
                zib.src_b("imm", i.imme as u64 ^ M64, false);
                zib.op("and").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, r{} # rd=0 imm!=0",
                    i.inst, i.rd, i.csr, i.rs1
                ));
                zib.j(4, 4);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        } else if i.imme == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("imm", 0, false);
            zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
            zib.op("copyb").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, 0x{:x}, r{} # rd!=0 imm=0", i.inst, i.rd, i.csr, i.rs1));
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
        } else {
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", 0, false);
                zib.src_b("mem", CSR_ADDR + i.csr as u64, false);
                zib.op("copyb").unwrap();
                zib.store("reg", i.rd as i64, false, false);
                zib.j(1, 1);
                zib.verbose(&format!(
                    "{} r{}, 0x{:x}, r{} # rd!=0 imm!=0",
                    i.inst, i.rd, i.csr, i.rs1
                ));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
            {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("lastc", 0, false);
                zib.src_b("imm", i.imme as u64 ^ M64, false);
                zib.op("and").unwrap();
                zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
                zib.j(3, 3);
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            }
        }
    }
} // impl Riscv2ZiskContext

/// Converts a buffer with RISC-V data into a vector of Zisk instructions, using the
/// Riscv2ZiskContext to perform the instruction transpilation
pub fn add_zisk_code(rom: &mut ZiskRom, addr: u64, data: &[u8]) {
    //print!("add_zisk_code() addr={}\n", addr);

    // Convert input data to a u32 vector
    let code_vector: Vec<u32> = convert_vector(data);

    // Convert data vector to RISCV instructions
    let riscv_instructions = riscv_interpreter(&code_vector);

    // Create a context to convert RISCV instructions to ZisK instructions, using rom.insts
    let mut ctx = Riscv2ZiskContext { s: addr, insts: &mut rom.insts };

    // For all RISCV instructions
    for riscv_instruction in riscv_instructions {
        //print!("add_zisk_code() converting RISCV instruction={}\n",
        // riscv_instruction.to_string());

        // Convert RICV instruction to ZisK instruction and store it in rom.insts
        ctx.convert(&riscv_instruction);
        //print!("   to: {}", ctx.insts.iter().last().)
    }
}

/// Add initial data to ZisK rom.
///
/// The initial data is copied in chunks of 8 bytes for efficiency, until less than 8 bytes are left
/// to copy.  The remaining bytes are copied in additional chunks of 4, 2 and 1 byte, if required.
pub fn add_zisk_init_data(rom: &mut ZiskRom, addr: u64, data: &[u8], force_aligned: bool) {
    /*let mut s = String::new();
    for i in 0..min(50, data.len()) {
        s += &format!("{:02x}", data[i]);
    }
    print!("add_zisk_init_data() addr={:x} len={} data={}...\n", addr, data.len(), s);*/

    let mut o = addr;

    // Read 64-bit input data chunks and store them in rom
    let nd = data.len() / 8;
    for i in 0..nd {
        let v = u64::from_le_bytes(data[i * 8..i * 8 + 8].try_into().unwrap());
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", o, false);
        zib.src_b("imm", v, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("ind", 0, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Init Data {o:08x}: {v:08x}"));
        zib.build();
        rom.insts.insert(rom.next_init_inst_addr, zib);
        rom.next_init_inst_addr += 4;
        o += 8;
    }

    // TODO: review if necessary
    let bytes = addr + data.len() as u64 - o;
    // If force_aligned is active always store aligned
    if force_aligned && bytes > 0 {
        let mut v: u64 = 0;
        let from = (o - addr + bytes - 1) as usize;
        for i in 0..bytes {
            v = v * 256 + data[from - i as usize] as u64;
        }
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", o, false);
        zib.src_b("imm", v, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("ind", 0, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Init Data {o:08x}: {v:04x}"));
        zib.build();
        rom.insts.insert(rom.next_init_inst_addr, zib);
        rom.next_init_inst_addr += 4;
        o += bytes;
    }

    // Read remaining 32-bit input data chunk, if any, and store them in rom
    if addr + data.len() as u64 - o >= 4 {
        let v = u32::from_le_bytes(data[o as usize..o as usize + 4].try_into().unwrap());
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", o, false);
        zib.src_b("imm", v as u64, false);
        zib.op("copyb").unwrap();
        zib.ind_width(4);
        zib.store("ind", 0, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Init Data {o:08x}: {v:04x}"));
        zib.build();
        rom.insts.insert(rom.next_init_inst_addr, zib);
        rom.next_init_inst_addr += 4;
        o += 4;
    }

    // Read remaining 16-bit input data chunk, if any, and store them in rom
    if addr + data.len() as u64 - o >= 2 {
        let v = u16::from_le_bytes(data[o as usize..o as usize + 2].try_into().unwrap());
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", o, false);
        zib.src_b("imm", v as u64, false);
        zib.op("copyb").unwrap();
        zib.ind_width(2);
        zib.store("ind", 0, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Init Data {o:08x}: {v:02x}"));
        zib.build();
        rom.insts.insert(rom.next_init_inst_addr, zib);
        rom.next_init_inst_addr += 4;
        o += 2;
    }

    // Read remaining 8-bit input data chunk, if any, and store them in rom
    if addr + data.len() as u64 - o >= 1 {
        let v = data[(o - addr) as usize];
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", o, false);
        zib.src_b("imm", v as u64, false);
        zib.op("copyb").unwrap();
        zib.ind_width(2);
        zib.store("ind", 0, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Init Data {o:08x}: {v:x}"));
        zib.build();
        rom.insts.insert(rom.next_init_inst_addr, zib);
        rom.next_init_inst_addr += 4;
        o += 1;
    }
    /*
        if force_aligned {
            let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
            zib.src_a("imm", o, false);
            zib.src_b("imm", 0, false);
            zib.op("copyb").unwrap();
            zib.ind_width(8);
            zib.store("ind", 0, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("Init Data {:08x}: {:04x}", o, 0));
            zib.build();
            rom.insts.insert(rom.next_init_inst_addr, zib);
            rom.next_init_inst_addr += 4;
        }
    */
    // Check resulting length
    if o != addr + data.len() as u64 {
        panic!("add_zisk_init_data() invalid length o={} addr={} data.len={}", o, addr, data.len());
    }
}

/// Add the entry/exit jump program section to the rom instruction set.
pub fn add_entry_exit_jmp(rom: &mut ZiskRom, addr: u64) {
    //print!("add_entry_exit_jmp() rom.next_init_inst_addr={}\n", rom.next_init_inst_addr);

    // Calculate the trap handler rom pc address as an offset from the current instruction address
    // to the beginning of the ecall section
    let trap_handler: u64 = rom.next_init_inst_addr + 0x38;

    // :0000 we note the rom pc address offset from the first address for each instruction
    // Store the Zisk architecture ID into memory
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", ARCH_ID_ZISK, false);
    zib.op("copyb").unwrap();
    zib.store("mem", CSR_ADDR as i64 + 0xF12, false, false);
    zib.j(4, 4);
    zib.verbose(&format!("Set marchid: {ARCH_ID_ZISK:x}"));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0004
    // Store the trap handler address into memory
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", trap_handler, false);
    zib.op("copyb").unwrap();
    zib.store("mem", MTVEC as i64, false, false);
    zib.j(4, 4);
    zib.verbose(&format!("Set mtvec: {trap_handler}"));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0008
    // Store the input data address into register #10
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", INPUT_ADDR, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 10, false, false);
    zib.j(0, 4);
    zib.verbose(&format!("Set 1st Param (pInput): 0x{INPUT_ADDR:08x}"));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :000c
    // Store the output data address into register #11
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", OUTPUT_ADDR, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 11, false, false);
    zib.j(0, 4);
    zib.verbose(&format!("Set 2nd Param (pOutput): 0x{OUTPUT_ADDR:08x}"));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0010
    // Call to the program rom pc address, i.e. call the program
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", addr, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.store_ra("reg", 1, false);
    zib.j(0, 4);
    zib.verbose(&format!("CALL to entry: 0x{addr:08x}"));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0014
    // Returns from the program execution.
    // Reads output data using the specific pubout operation in 32 chunks of 64 bits:
    //
    // loadw: c(reg11) = b(32), a=0
    // copyb: c(reg12)=b=0, a=0
    // copyb: c(reg13)=b=OUTPUT_ADDR, a=0
    //
    // eq: if reg12==reg11 jump to end
    // pubout: c=b.mem(reg13), a = reg12
    // add: reg13 = reg13 + 8 // Increment memory address
    // add: reg12 = reg12 + 1, jump -12 // Increment index, goto eq
    //
    // end
    //
    // Copy output data address into register #1
    // copyb: reg11 = c = b = mem(OUTPUT_ADDR,4), a=0
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 32, false);
    zib.ind_width(4);
    zib.op("copyb").unwrap();
    zib.store("reg", 11, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg11 to output data length = 32");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0018 -> copyb: copyb: c(reg12)=b=0, a=0
    // Set register #12 to zero
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 12, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg12 to 0");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :001c -> copyb: c(reg13)=b=OUTPUT_ADDR, a=0
    // Set register #13 to OUTPUT_ADDR, i.e. to the beginning of the actual data after skipping
    // the data length value
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", OUTPUT_ADDR, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 13, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg13 to OUTPUT_ADDR");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0020 -> eq: if reg12==reg11 jump to end
    // Jump to end if registers #11 and #12 are equal, to break the data copy loop
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 11, false);
    zib.src_b("reg", 12, false);
    zib.op("eq").unwrap();
    zib.store("none", 0, false, false);
    zib.j(20, 4);
    zib.verbose("If reg11==reg12 jumpt to end");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0024 -> copyb: c = b = mem(reg13, 8)
    // Copy the contents of memory at address set by register #13 into c, i.e. copy output data chunk
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 13, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.store("none", 0, false, false);
    zib.j(0, 4);
    zib.verbose("Set c to mem(output_data[index]), a=index");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0028 -> pubout: c = last_c = mem(reg13, 8), a = reg12 = index
    // Call the special operation pubout with this data, being a the data chunk index
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 12, false);
    zib.src_b("lastc", 0, false);
    zib.op("pubout").unwrap();
    zib.store("none", 0, false, false);
    zib.j(0, 4);
    zib.verbose("Public output, set c to output_data[index], a=index");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :002c -> add: reg13 = reg13 + 8
    // Increase the register #13, i.e. the data address, in 8 units
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 13, false);
    zib.src_b("imm", 8, false);
    zib.op("add").unwrap();
    zib.store("reg", 13, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg13 to reg13 + 8");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0030 -> add: reg12 = reg12 + 1, jump -16
    // Increase the register #12, i.e. the data chunk index, in 1 unit.
    // Jump to the beginning of the output data read loop
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 12, false);
    zib.src_b("imm", 1, false);
    zib.op("add").unwrap();
    zib.store("reg", 12, false, false);
    zib.j(4, -16);
    zib.verbose("Set reg12 to reg12 + 1");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0034 jump to end (success)
    // Jump to the last instruction (ROM_EXIT) to properly finish the program execution
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", ROM_EXIT, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.j(0, 0);
    zib.verbose("jump to end successfully");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0038 trap_handle -> This is the address offset we use at the beginning of the function
    // This code is executed when the program makes an ecall (system call).
    // The pc is set to this address, and after the system call, it returns to the pc next to the
    // one that made the ecall
    // If register a7==CAUSE_EXIT, then execute the next instruction to end the program;
    // otherwise jump to the one after the next one
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 17, false);
    zib.src_b("imm", CAUSE_EXIT, false);
    zib.op("eq").unwrap();
    zib.j(-36, 4);
    zib.verbose(&format!("beq r17, {CAUSE_EXIT} # Check if is exit, jump to output, then end"));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :003c
    // Return to the instruction next to the one that made this ecall
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("reg", 1, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.j(0, 4);
    zib.verbose("ret");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;
}

/// Add the end jump program section to the rom instruction set.
pub fn add_end_jmp(rom: &mut ZiskRom) {
    //print!("add_entry_exit_jmp() rom.next_init_inst_addr={}\n", rom.next_init_inst_addr);

    // :0000 we jump to the third instruction, leaving room for the end instruction
    assert!(rom.next_init_inst_addr == ROM_ENTRY);
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    zib.j(8, 8);
    zib.verbose("Jump over end instruction");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0004 END: all programs should exit here, regardless of the execution result
    // This is the last instruction to be executed.  The emulator must stop after the instruction
    // end flag is found to be true
    assert!(rom.next_init_inst_addr == ROM_EXIT);
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    zib.end();
    zib.j(0, 0);
    zib.verbose("end");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;
}
