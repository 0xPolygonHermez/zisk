use riscv::{riscv_interpreter, RiscvInstruction};

use crate::{
    convert_vector, read_u16_le, read_u32_le, read_u64_le, ZiskInstBuilder, ZiskRom, ARCH_ID_ZISK,
    INPUT_ADDR, OUTPUT_ADDR, ROM_EXIT, SYS_ADDR,
};

use std::collections::HashMap;

const CAUSE_EXIT: u64 = 93;
const CAUSE_KECCAK: u64 = 0x00_01_01_01;
const CSR_ADDR: u64 = SYS_ADDR + 0x8000;
const MTVEC: u64 = CSR_ADDR + 0x305;
const M64: u64 = 0xFFFFFFFFFFFFFFFF;

/// Context to store the list of converted ZisK instructions, including their program address
struct Riscv2ZiskContext<'a> {
    /// Next program address to assign
    s: u64,
    /// Map of program address to ZisK instructions
    pub insts: &'a mut HashMap<u64, ZiskInstBuilder>,
}

impl Riscv2ZiskContext<'_> {
    /// Converts an input RISCV instruction into a ZisK instruction and stores it into the internal
    /// map
    pub fn convert(&mut self, riscv_instruction: &RiscvInstruction) {
        //let mut addr = self.s;
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
            "addi" => self.immediate_op(riscv_instruction, "add"),
            "slli" => self.immediate_op(riscv_instruction, "sll"),
            "slti" => self.immediate_op(riscv_instruction, "lt"),
            "sltiu" => self.immediate_op(riscv_instruction, "ltu"),
            "xori" => self.immediate_op(riscv_instruction, "xor"),
            "srli" => self.immediate_op(riscv_instruction, "srl"),
            "srai" => self.immediate_op(riscv_instruction, "sra"),
            "ori" => self.immediate_op(riscv_instruction, "or"),
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

        /*if self.insts.contains_key(&addr)
        {
            let zib = self.insts.get(&addr).unwrap();
            println!("Riscv2ZiskContext::convert() addr={} inst={}", addr, zib.i.to_string());
        }
        addr += 1;
        if self.insts.contains_key(&addr)
        {
            let zib = self.insts.get(&addr).unwrap();
            println!("Riscv2ZiskContext::convert() addr={} inst={}", addr, zib.i.to_string());
        }
        addr += 1;
        if self.insts.contains_key(&addr)
        {
            let zib = self.insts.get(&addr).unwrap();
            println!("Riscv2ZiskContext::convert() addr={} inst={}", addr, zib.i.to_string());
        }
        addr += 1;
        if self.insts.contains_key(&addr)
        {
            let zib = self.insts.get(&addr).unwrap();
            println!("Riscv2ZiskContext::convert() addr={} inst={}", addr, zib.i.to_string());
        }*/
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
                //zib.ind_width(w);
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

    pub fn ecall(&mut self, _i: &RiscvInstruction) {
        let mut zib = ZiskInstBuilder::new(self.s);
        zib.src_a("imm", 0, false);
        zib.src_b("mem", MTVEC, false);
        zib.op("copyb").unwrap();
        //zib.store_ra("reg", 1, false);
        zib.set_pc();
        zib.j(0, 4);
        zib.verbose("ecall");
        zib.build();
        self.insts.insert(self.s, zib);
        self.s += 4;
    }

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

    pub fn csrrw(&mut self, i: &RiscvInstruction) {
        if i.rd == i.rs1 {
            if i.rd == 0 {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", 0, false);
                zib.src_b("mem", 0, false);
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
                    zib.src_a("mem", 0, false);
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
                    zib.src_a("mem", 0, false);
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
                    zib.src_a("mem", 0, false);
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
                zib.src_a("mem", 0, false);
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
                zib.src_a("mem", 0, false);
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

    pub fn csrrs(&mut self, i: &RiscvInstruction) {
        if i.rd == i.rs1 {
            if i.rd == 0 {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", 0, false);
                zib.src_b("mem", 0, false);
                zib.op("copyb").unwrap();
                zib.j(4, 4);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} ## rd=rs=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            } else {
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("mem", 0, false);
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
                    zib.src_a("mem", 0, false);
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
            zib.src_a("mem", CSR_ADDR + i.csr as u64, false);
            zib.src_b("reg", i.rs1 as u64, false);
            zib.op("or").unwrap();
            zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, 0x{:x}, r{} # rs!=rd=0", i.inst, i.rd, i.csr, i.rs1));
            zib.build();
            self.insts.insert(self.s, zib);
            self.s += 4;
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

    pub fn csrrc(&mut self, i: &RiscvInstruction) {
        if i.rd == i.rs1 {
            if i.rd == 0 {
                let mut zib = ZiskInstBuilder::new(self.s);
                zib.src_a("mem", 0, false);
                zib.src_b("mem", 0, false);
                zib.op("copyb").unwrap();
                zib.j(4, 4);
                zib.verbose(&format!("{} r{}, 0x{:x}, r{} ## rd=rs=0", i.inst, i.rd, i.csr, i.rs1));
                zib.build();
                self.insts.insert(self.s, zib);
                self.s += 4;
            } else {
                {
                    let mut zib = ZiskInstBuilder::new(self.s);
                    zib.src_a("mem", 0, false);
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
    pub fn csrrwi(&mut self, i: &RiscvInstruction) {
        if i.rd == 0 {
            let mut zib = ZiskInstBuilder::new(self.s);
            zib.src_a("imm", 0, false);
            zib.src_b("imm", i.imme as u64, false);
            zib.op("copyb").unwrap();
            zib.store("mem", CSR_ADDR as i64 + i.csr as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, 0x{:x}, 0x{:x} #rd = 0", i.inst, i.rd, i.csr, i.imme));
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

/// Converts a buffer with RISCV data into a vector of Zisk instructions
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
        // riscv_instruction.to_string()); Convert RICV instruction to ZisK instruction and
        // store it in rom.insts
        ctx.convert(&riscv_instruction);
        //print!("   to: {}", ctx.insts.iter().last().)
    }
}

/// Add initial data to ZisK rom
pub fn add_zisk_init_data(rom: &mut ZiskRom, addr: u64, data: &[u8]) {
    /*let mut s = String::new();
    for i in 0..min(50, data.len()) {
        s += &format!("{:02x}", data[i]);
    }
    print!("add_zisk_init_data() addr={:x} len={} data={}...\n", addr, data.len(), s);*/

    let mut o = addr;

    // Read 64-bit input data chunks and store them in rom
    let nd = data.len() / 8;
    for i in 0..nd {
        let v = read_u64_le(data, i * 8);
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", o, false);
        zib.src_b("imm", v, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("ind", 0, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Init Data {:08x}: {:08x}", o, v));
        zib.build();
        rom.insts.insert(rom.next_init_inst_addr, zib);
        rom.next_init_inst_addr += 4;
        o += 8;
    }

    // Read remaining 32-bit input data chunk, if any, and store them in rom
    if addr + data.len() as u64 - o >= 4 {
        let v = read_u32_le(data, (o - addr) as usize);
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", o, false);
        zib.src_b("imm", v as u64, false);
        zib.op("copyb").unwrap();
        zib.ind_width(4);
        zib.store("ind", 0, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Init Data {:08x}: {:04x}", o, v));
        zib.build();
        rom.insts.insert(rom.next_init_inst_addr, zib);
        rom.next_init_inst_addr += 4;
        o += 4;
    }

    // Read remaining 16-bit input data chunk, if any, and store them in rom
    if addr + data.len() as u64 - o >= 2 {
        let v = read_u16_le(data, (o - addr) as usize);
        let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
        zib.src_a("imm", o, false);
        zib.src_b("imm", v as u64, false);
        zib.op("copyb").unwrap();
        zib.ind_width(2);
        zib.store("ind", 0, false, false);
        zib.j(4, 4);
        zib.verbose(&format!("Init Data {:08x}: {:02x}", o, v));
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
        zib.verbose(&format!("Init Data {:08x}: {:x}", o, v));
        zib.build();
        rom.insts.insert(rom.next_init_inst_addr, zib);
        rom.next_init_inst_addr += 4;
        o += 1;
    }

    // Check resulting length
    if o != addr + data.len() as u64 {
        panic!("add_zisk_init_data() invalid length o={} addr={} data.len={}", o, addr, data.len());
    }
}

/// Add the entry/exit jump program section
pub fn add_entry_exit_jmp(rom: &mut ZiskRom, addr: u64) {
    //print!("add_entry_exit_jmp() rom.next_init_inst_addr={}\n", rom.next_init_inst_addr);
    let trap_handler: u64 = rom.next_init_inst_addr + 0x38;

    // :0000
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", ARCH_ID_ZISK, false);
    zib.op("copyb").unwrap();
    zib.store("mem", CSR_ADDR as i64 + 0xF12, false, false);
    zib.j(4, 4);
    zib.verbose(&format!("Set marchid: {:x}", ARCH_ID_ZISK));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0004
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", trap_handler, false);
    zib.op("copyb").unwrap();
    zib.store("mem", MTVEC as i64, false, false);
    zib.j(4, 4);
    zib.verbose(&format!("Set mtvec: {}", trap_handler));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0008
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", INPUT_ADDR, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 10, false, false);
    zib.j(0, 4);
    zib.verbose(&format!("Set 1st Param (pInput): 0x{:08x}", INPUT_ADDR));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :000c
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", OUTPUT_ADDR, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 11, false, false);
    zib.j(0, 4);
    zib.verbose(&format!("Set 2nd Param (pOutput): 0x{:08x}", OUTPUT_ADDR));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0010
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", addr, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.store_ra("reg", 1, false);
    zib.j(0, 4);
    zib.verbose(&format!("CALL to entry: 0x{:08x}", addr));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    /* Read output length located at first 64 bits of output data,
       then read output data in chunks of 64 bits:

            loadw: c(reg1) = b(mem=OUTPUT_ADDR), a=0   // TODO: check that Nx4 < OUTPUT_SIZE
            copyb: c(reg2)=b=0, a=0
            copyb: c(reg3)=b=OUTPUT_ADDR+4, a=0

            eq: if reg2==reg1 jump to end
            pubout: c=b.mem(reg3), a = reg2
            add: reg3 = reg3 + 4 // Increment memory address
            add: reg2 = reg2 + 1, jump -12 // Increment index, goto eq

            end
    */

    // :0014 -> copyb: reg1 = c = b = mem(OUTPUT_ADDR,4), a=0
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", OUTPUT_ADDR, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(4);
    zib.op("copyb").unwrap();
    zib.store("reg", 1, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg1 to output data length read at OUTPUT_ADDR");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0018 -> copyb: copyb: c(reg2)=b=0, a=0
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 2, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg2 to 0");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :001c -> copyb: c(reg3)=b=OUTPUT_ADDR, a=0
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", OUTPUT_ADDR + 4, false);
    zib.op("copyb").unwrap();
    zib.store("reg", 3, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg3 to OUTPUT_ADDR + 4");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0020 -> eq: if reg2==reg1 jump to end
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 1, false);
    zib.src_b("reg", 2, false);
    zib.op("eq").unwrap();
    zib.store("none", 0, false, false);
    zib.j(20, 4);
    zib.verbose("If reg1==reg2 jumpt to end");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0024 -> copyb: c = b = mem(reg3, 4)
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 3, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(4);
    zib.op("copyb").unwrap();
    zib.store("none", 0, false, false);
    zib.j(0, 4);
    zib.verbose("Set c to mem(output_data[index]), a=index");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0028 -> pubout: c = last_c = mem(reg3, 4), a = reg2 = index
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 2, false);
    zib.src_b("lastc", 0, false);
    zib.op("pubout").unwrap();
    zib.store("none", 0, false, false);
    zib.j(0, 4);
    zib.verbose("Public output, set c to output_data[index], a=index");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :002c -> add: reg3 = reg3 + 4
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 3, false);
    zib.src_b("imm", 4, false);
    zib.op("add").unwrap();
    zib.store("reg", 3, false, false);
    zib.j(0, 4);
    zib.verbose("Set reg3 to reg3 + 4");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0030 -> add: reg2 = reg2 + 1, jump -16
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 2, false);
    zib.src_b("imm", 1, false);
    zib.op("add").unwrap();
    zib.store("reg", 2, false, false);
    zib.j(4, -16);
    zib.verbose("Set reg2 to reg2 + 1");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0034 jump to end (success)
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

    // :0038 trap_handle
    // If register a7==CAUSE_EXIT, end the program
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 17, false);
    zib.src_b("imm", CAUSE_EXIT, false);
    zib.op("eq").unwrap();
    zib.j(4, 8);
    zib.verbose(&format!("beq r17, {} # Check if is exit", CAUSE_EXIT));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :003c jump to END (error)
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", ROM_EXIT, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.j(0, 0);
    zib.verbose("jump to end due to error");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0040 trap_handle
    // If register a7==CAUSE_KECCAK, call the keccak opcode and return
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 17, false);
    zib.src_b("imm", CAUSE_KECCAK, false);
    zib.op("eq").unwrap();
    zib.j(4, 8);
    zib.verbose(&format!("beq r17, {} # Check if is keccak", CAUSE_KECCAK));
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0044
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("reg", 11, false);
    zib.src_b("imm", 0, false);
    zib.op("keccak").unwrap();
    zib.j(4, 4);
    zib.verbose("keccak");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
    rom.next_init_inst_addr += 4;

    // :0048
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

    // END: all programs should exit here, regardless of the execution result
    rom.next_init_inst_addr = ROM_EXIT;
    let mut zib = ZiskInstBuilder::new(rom.next_init_inst_addr);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    zib.end();
    zib.j(0, 0);
    zib.verbose("end");
    zib.build();
    rom.insts.insert(rom.next_init_inst_addr, zib);
}
