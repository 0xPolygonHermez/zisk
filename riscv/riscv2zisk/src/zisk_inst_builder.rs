use crate::{
    ZiskInst, ZiskOperations, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_SP, SRC_STEP, STORE_IND,
    STORE_MEM, STORE_NONE, SYS_ADDR,
};

pub const INVALID_VALUE: u64 = 0xFFFFFFFFFFFFFFFF_u64;
pub const INVALID_VALUE_S64: i64 = 0xFFFFFFFFFFFFFFF_i64;
const INITIAL_VALUE: u64 = INVALID_VALUE;
const INITIAL_VALUE_S64: i64 = INVALID_VALUE_S64;

pub struct ZiskInstBuilder {
    ind_width_set: bool,
    pub i: ZiskInst,
    zisk_ops: ZiskOperations,
    regs_addr: u64,
}

impl ZiskInstBuilder {
    pub fn new(paddr: u64) -> ZiskInstBuilder {
        let zisk_ops = ZiskOperations::new();
        let regs_addr = SYS_ADDR;

        ZiskInstBuilder {
            ind_width_set: false,
            i: ZiskInst {
                paddr,
                store_ra: 0,
                store_use_sp: 0,
                store: STORE_NONE,
                store_offset: 0,
                set_pc: 0,
                set_sp: 0,
                ind_width: 8,
                inc_sp: 0,
                end: 0,
                a_src: INITIAL_VALUE,
                a_use_sp_imm1: INITIAL_VALUE,
                a_offset_imm0: INITIAL_VALUE,
                b_src: INITIAL_VALUE,
                b_use_sp_imm1: INITIAL_VALUE,
                b_offset_imm0: INITIAL_VALUE,
                jmp_offset1: INITIAL_VALUE_S64,
                jmp_offset2: INITIAL_VALUE_S64,
                is_external_op: INITIAL_VALUE,
                op: 0,
                op_str: "",
                verbose: String::new(),
            },
            zisk_ops,
            regs_addr,
        }
    }

    fn a_src(&self, src: &str) -> u64 {
        match src {
            "mem" => SRC_MEM,
            "imm" => SRC_IMM,
            "lastc" => SRC_C,
            "sp" => SRC_SP,
            "step" => SRC_STEP,
            _ => panic!("ZiskInstBuilder::a_src() called with invalid src={}", src),
        }
    }

    fn b_src(&self, src: &str) -> u64 {
        match src {
            "mem" => SRC_MEM,
            "imm" => SRC_IMM,
            "lastc" => SRC_C,
            "ind" => SRC_IND,
            _ => panic!("ZiskInstBuilder::b_src() called with invalid src={}", src),
        }
    }

    fn c_store(&self, store: &str) -> u64 {
        match store {
            "none" => STORE_NONE,
            "mem" => STORE_MEM,
            "ind" => STORE_IND,
            _ => panic!("ZiskInstBuilder::c_store() called with invalid store={}", store),
        }
    }

    pub fn nto32s(n: i128) -> (u32, u32) {
        let mut a = n;
        if a >= (1_i128 << 64) {
            panic!("ZiskInstBuilder::nto32s() n={} is too big", a);
        }
        if a < 0 {
            a += 1_i128 << 64;
            if a < (1_i128 << 63) {
                panic!("ZiskInstBuilder::nto32s() n={} is too small", a);
            }
        }
        ((a & 0xFFFFFFFF) as u32, (a >> 32) as u32)
    }

    pub fn src_a(&mut self, src_input: &str, offset_imm_reg_input: u64, use_sp: bool) {
        let mut src = src_input;
        let mut offset_imm_reg = offset_imm_reg_input;
        if src == "reg" {
            if offset_imm_reg == 0 {
                src = "imm";
                offset_imm_reg = 0;
            } else {
                src = "mem";
                offset_imm_reg = self.regs_addr + offset_imm_reg * 8;
            }
        }
        self.i.a_src = self.a_src(src);

        if self.i.a_src == SRC_MEM {
            if use_sp {
                self.i.a_use_sp_imm1 = 1;
            } else {
                self.i.a_use_sp_imm1 = 0;
            }
            self.i.a_offset_imm0 = offset_imm_reg;
        } else if self.i.a_src == SRC_IMM {
            let (v0, v1) = Self::nto32s(offset_imm_reg as i128);
            self.i.a_use_sp_imm1 = v1 as u64;
            self.i.a_offset_imm0 = v0 as u64;
        } else {
            self.i.a_use_sp_imm1 = 0;
            self.i.a_offset_imm0 = 0;
        }
    }

    pub fn src_b(&mut self, src_input: &str, offset_imm_reg_input: u64, use_sp: bool) {
        let mut src = src_input;
        let mut offset_imm_reg = offset_imm_reg_input;
        if src == "reg" {
            if offset_imm_reg == 0 {
                src = "imm";
                offset_imm_reg = 0;
            } else {
                src = "mem";
                offset_imm_reg = self.regs_addr + offset_imm_reg * 8;
            }
        }
        self.i.b_src = self.b_src(src);

        if self.i.b_src == SRC_MEM || self.i.b_src == SRC_IND {
            if use_sp {
                self.i.b_use_sp_imm1 = 1;
            } else {
                self.i.b_use_sp_imm1 = 0;
            }
            self.i.b_offset_imm0 = offset_imm_reg;
        } else if self.i.b_src == SRC_IMM {
            let (v0, v1) = Self::nto32s(offset_imm_reg as i128);
            self.i.b_use_sp_imm1 = v1 as u64;
            self.i.b_offset_imm0 = v0 as u64;
        } else {
            self.i.b_use_sp_imm1 = 0;
            self.i.b_offset_imm0 = 0;
        }
    }

    pub fn store(&mut self, dst_input: &str, offset_input: i64, use_sp: bool, store_ra: bool) {
        let mut dst = dst_input;
        let mut offset = offset_input;
        if dst == "reg" {
            if offset == 0 {
                return;
            } else {
                dst = "mem";
                offset = self.regs_addr as i64 + offset * 8;
            }
        }

        if store_ra {
            self.i.store_ra = 1;
        } else {
            self.i.store_ra = 0;
        }
        self.i.store = self.c_store(dst);

        if self.i.store == STORE_MEM || self.i.store == STORE_IND {
            if use_sp {
                self.i.store_use_sp = 1;
            } else {
                self.i.store_use_sp = 0;
            }
            self.i.store_offset = offset;
        } else {
            self.i.store_use_sp = 0;
            self.i.store_offset = 0;
        }
    }

    pub fn store_ra(&mut self, dst: &str, offset: i64, use_sp: bool) {
        self.store(dst, offset, use_sp, true);
    }

    pub fn set_pc(&mut self) {
        self.i.set_pc = 1;
    }

    pub fn set_sp(&mut self) {
        self.i.set_sp = 1;
    }

    pub fn op(&mut self, optxt: &str) {
        let op = self.zisk_ops.op_from_str.get(optxt).unwrap();
        if op.t == "i" {
            self.i.is_external_op = 0;
        } else if op.t == "e" {
            self.i.is_external_op = 1;
        } else {
            panic!("ZiskInstBuilder::op() found invalid op={}", optxt);
        }
        self.i.op = op.c;
        self.i.op_str = op.n;
    }

    pub fn j(&mut self, j1: i32, j2: i32) {
        self.i.jmp_offset1 = j1 as i64;
        self.i.jmp_offset2 = j2 as i64;
    }

    pub fn check(&self) {
        if self.i.a_src == INVALID_VALUE {
            panic!("ZiskInstBuilder::check() found a_src={}", self.i.a_src);
        }
        if self.i.a_use_sp_imm1 == INVALID_VALUE {
            panic!("ZiskInstBuilder::check() found a_use_sp_imm1={}", self.i.a_use_sp_imm1);
        }
        if self.i.a_offset_imm0 == INVALID_VALUE {
            panic!("ZiskInstBuilder::check() found a_offset_imm0={}", self.i.a_offset_imm0);
        }
        if self.i.b_src == INVALID_VALUE {
            panic!("ZiskInstBuilder::check() found b_src={}", self.i.b_src);
        }
        //if self.i.store_ra == INVALID_VALUE { panic!("ZiskInstBuilder::check() found
        // store_ra={}", self.i.store_ra); } if self.i.store == INVALID_VALUE {
        // panic!("ZiskInstBuilder::check() found store={}", self.i.store); }
        // if self.i.set_sp == INVALID_VALUE { panic!("ZiskInstBuilder::check() found set_sp={}",
        // self.i.set_sp); } if self.i.store_use_sp == INVALID_VALUE {
        // panic!("ZiskInstBuilder::check() found store_use_sp={}", self.i.store_use_sp); }
        // if self.i.store_offset == INVALID_VALUE { panic!("ZiskInstBuilder::check() found
        // store_offset={}", self.i.store_offset); } if self.i.ind_width == INVALID_VALUE {
        // panic!("ZiskInstBuilder::check() found ind_width={}", self.i.ind_width); }
        if self.i.is_external_op == INVALID_VALUE {
            panic!("ZiskInstBuilder::check() found is_external_op={}", self.i.is_external_op);
        }
        //if self.i.op == INVALID_VALUE {
        //    panic!("ZiskInstBuilder::check() found op={}", self.i.op);
        //}
        //if self.i.inc_sp == INVALID_VALUE { panic!("ZiskInstBuilder::check() found inc_sp={}",
        // self.i.inc_sp); }
        if self.i.jmp_offset1 == INVALID_VALUE as i64 {
            panic!("ZiskInstBuilder::check() found jmp_offset1={}", self.i.jmp_offset1);
        }
        if self.i.jmp_offset2 == INVALID_VALUE as i64 {
            panic!("ZiskInstBuilder::check() found jmp_offset2={}", self.i.jmp_offset2);
        }
        if self.i.end == INVALID_VALUE {
            panic!("ZiskInstBuilder::check() found end={}", self.i.end);
        }

        if (self.i.b_src == SRC_IND) && (self.i.store == STORE_IND) {
            panic!("ZiskInstBuilder::check() Load and store cannot bi indirect at the same time");
        }

        if ((self.i.b_src == SRC_IND) || (self.i.store == STORE_IND)) && !self.ind_width_set {
            panic!("ZiskInstBuilder::check() indWidthSet must be set in indirect access")
        }
    }

    pub fn ind_width(&mut self, w: u64) {
        if w != 1 && w != 2 && w != 4 && w != 8 {
            panic!("ZiskInstBuilder::indWidth() invalid v={}", w);
        }
        self.i.ind_width = w;
        self.ind_width_set = true;
    }

    pub fn end(&mut self) {
        self.i.end = 1;
    }

    pub fn inc_sp(&mut self, inc: u64) {
        self.i.inc_sp += inc;
    }

    pub fn verbose(&mut self, s: &str) {
        self.i.verbose = s.to_owned();
    }

    pub fn build(&self) {
        //print!("ZiskInstBuilder::build() i=[ {} ]\n", self.i.to_string());
        self.check();
    }
}
