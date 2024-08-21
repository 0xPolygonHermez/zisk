use crate::{
    op_from_str, ZiskInst, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_STEP, STORE_IND, STORE_MEM,
    STORE_NONE, SYS_ADDR,
};

#[cfg(feature = "sp")]
use crate::SRC_SP;

#[derive(Debug)]
pub struct ZiskInstBuilder {
    ind_width_set: bool,
    pub i: ZiskInst,
    regs_addr: u64,
}

impl ZiskInstBuilder {
    pub fn new(paddr: u64) -> ZiskInstBuilder {
        let regs_addr = SYS_ADDR;

        ZiskInstBuilder {
            ind_width_set: false,
            i: ZiskInst {
                paddr,
                store_ra: false,
                store_use_sp: false,
                store: STORE_NONE,
                store_offset: 0,
                set_pc: false,
                #[cfg(feature = "sp")]
                set_sp: false,
                ind_width: 8,
                #[cfg(feature = "sp")]
                inc_sp: 0,
                end: false,
                a_src: 0,
                a_use_sp_imm1: 0,
                a_offset_imm0: 0,
                b_src: 0,
                b_use_sp_imm1: 0,
                b_offset_imm0: 0,
                jmp_offset1: 0,
                jmp_offset2: 0,
                is_external_op: false,
                op: 0,
                func: |_, _| (0, false),
                op_str: "",
                verbose: String::new(),
                m32: false,
            },
            regs_addr,
        }
    }

    fn a_src(&self, src: &str) -> u64 {
        match src {
            "mem" => SRC_MEM,
            "imm" => SRC_IMM,
            "lastc" => SRC_C,
            #[cfg(feature = "sp")]
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

        self.i.store_ra = store_ra;
        self.i.store = self.c_store(dst);

        if self.i.store == STORE_MEM || self.i.store == STORE_IND {
            self.i.store_use_sp = use_sp;
            self.i.store_offset = offset;
        } else {
            self.i.store_use_sp = false;
            self.i.store_offset = 0;
        }
    }

    pub fn store_ra(&mut self, dst: &str, offset: i64, use_sp: bool) {
        self.store(dst, offset, use_sp, true);
    }

    pub fn set_pc(&mut self) {
        self.i.set_pc = true;
    }

    #[cfg(feature = "sp")]
    pub fn set_sp(&mut self) {
        self.i.set_sp = true;
    }

    pub fn op(&mut self, optxt: &str) {
        let op = op_from_str(optxt);
        self.i.is_external_op = op.t != "i";
        self.i.op = op.c;
        self.i.op_str = op.n;
        self.i.m32 = optxt.contains("_w");
    }

    pub fn j(&mut self, j1: i32, j2: i32) {
        self.i.jmp_offset1 = j1 as i64;
        self.i.jmp_offset2 = j2 as i64;
    }

    pub fn ind_width(&mut self, w: u64) {
        if w != 1 && w != 2 && w != 4 && w != 8 {
            panic!("ZiskInstBuilder::indWidth() invalid v={}", w);
        }
        self.i.ind_width = w;
        self.ind_width_set = true;
    }

    pub fn end(&mut self) {
        self.i.end = true;
    }

    #[cfg(feature = "sp")]
    pub fn inc_sp(&mut self, inc: u64) {
        self.i.inc_sp += inc;
    }

    pub fn verbose(&mut self, s: &str) {
        self.i.verbose = s.to_owned();
    }

    pub fn build(&mut self) {
        //print!("ZiskInstBuilder::build() i=[ {} ]\n", self.i.to_string());
        self.i.func = op_from_str(self.i.op_str).f;
    }
}
