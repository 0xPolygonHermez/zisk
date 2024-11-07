use crate::{
    source_to_str, store_to_str, InstContext, SRC_IMM, SRC_IND, SRC_MEM, SRC_STEP, STORE_IND,
    STORE_MEM,
};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(u32)]
pub enum ZiskOperationType {
    None,
    Internal,
    Arith,
    Binary,
    BinaryE,
    Keccak,
    PubOut,
}

pub const ZISK_OPERATION_TYPE_VARIANTS: usize = 6;

/// ZisK instruction definition
///
/// ZisK instruction defined as a binary operation with 2 results: op(a, b) -> (c, flag)
/// a and b are loaded from the respective sources specified in the instruction
/// c is stored according to the destination specified in the instruction
/// flag can only be 0 or 1
#[derive(Debug, Clone)]
pub struct ZiskInst {
    pub paddr: u64,
    pub store_ra: bool,
    pub store_use_sp: bool,
    pub store: u64,
    pub store_offset: i64,
    pub set_pc: bool,
    // #[cfg(feature = "sp")]
    // pub set_sp: bool,
    pub ind_width: u64,
    // #[cfg(feature = "sp")]
    // pub inc_sp: u64,
    pub end: bool,
    pub a_src: u64,
    pub a_use_sp_imm1: u64,
    pub a_offset_imm0: u64,
    pub b_src: u64,
    pub b_use_sp_imm1: u64,
    pub b_offset_imm0: u64,
    pub jmp_offset1: i64,
    pub jmp_offset2: i64,
    pub is_external_op: bool,
    pub op: u8,
    pub func: fn(&mut InstContext) -> (),
    pub op_str: &'static str,
    pub op_type: ZiskOperationType,
    pub verbose: String,
    pub m32: bool,
}

/// Default constructor
/// Initializes all fields to 0
impl Default for ZiskInst {
    fn default() -> Self {
        Self {
            paddr: 0,
            store_ra: false,
            store_use_sp: false,
            store: 0,
            store_offset: 0,
            set_pc: false,
            // #[cfg(feature = "sp")]
            // set_sp: false,
            ind_width: 0,
            // #[cfg(feature = "sp")]
            // inc_sp: 0,
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
            func: |_| (),
            op_str: "",
            op_type: ZiskOperationType::None,
            verbose: String::new(),
            m32: false,
        }
    }
}

/// ZisK instruction implementation
impl ZiskInst {
    /// Creates a human-readable string containing the ZisK instruction fields that are not zero
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        if self.paddr != 0 {
            s += &(" paddr=".to_string() + &self.paddr.to_string());
        }
        if self.store_ra {
            s += &(" store_ra=".to_string() + &self.store_ra.to_string());
        }
        if self.store_use_sp {
            s += &(" store_use_sp=".to_string() + &self.store_use_sp.to_string());
        }
        if self.store != 0 {
            s += &format!(" store={}={}", self.store, store_to_str(self.store));
        }
        if self.store_offset != 0 {
            s += &(" store_offset=".to_string() + &self.store_offset.to_string());
        }
        if self.set_pc {
            s += &(" set_pc=".to_string() + &self.set_pc.to_string());
        }
        // #[cfg(feature = "sp")]
        // if self.set_sp {
        //     s += &(" set_sp=".to_string() + &self.set_sp.to_string());
        // }
        if self.ind_width != 0 {
            s += &(" ind_width=".to_string() + &self.ind_width.to_string());
        }
        // #[cfg(feature = "sp")]
        // if self.inc_sp != 0 {
        //     s += &(" inc_sp=".to_string() + &self.inc_sp.to_string());
        // }
        if self.end {
            s += &(" end=".to_string() + &self.end.to_string());
        }
        if self.a_src != 0 {
            s += &format!(" a_src={}={}", self.a_src, source_to_str(self.a_src));
        }
        if self.a_use_sp_imm1 != 0 {
            s += &(" a_use_sp_imm1=".to_string() + &self.a_use_sp_imm1.to_string());
        }
        if self.a_offset_imm0 != 0 {
            s += &(" a_offset_imm0=".to_string() + &self.a_offset_imm0.to_string());
        }
        if self.b_src != 0 {
            s += &format!(" b_src={}={}", self.b_src, source_to_str(self.b_src));
        }
        if self.b_use_sp_imm1 != 0 {
            s += &(" b_use_sp_imm1=".to_string() + &self.b_use_sp_imm1.to_string());
        }
        if self.b_offset_imm0 != 0 {
            s += &(" b_offset_imm0=".to_string() + &self.b_offset_imm0.to_string());
        }
        if self.jmp_offset1 != 0 {
            s += &(" jmp_offset1=".to_string() + &self.jmp_offset1.to_string());
        }
        if self.jmp_offset2 != 0 {
            s += &(" jmp_offset2=".to_string() + &self.jmp_offset2.to_string());
        }
        if self.is_external_op {
            s += &(" is_external_op=".to_string() + &self.is_external_op.to_string());
        }
        {
            s += &(" op=".to_string() + &self.op.to_string() + "=" + self.op_str);
        }
        if self.m32 {
            s += &(" m32=".to_string() + &self.m32.to_string());
        }
        if !self.verbose.is_empty() {
            s += &(" verbose=".to_string() + &self.verbose);
        }
        s
    }

    pub fn get_flags(&self) -> u64 {
        let flags: u64 = 1
            | (((self.a_src == SRC_IMM) as u64) << 1)
            | (((self.a_src == SRC_MEM) as u64) << 2)
            | (((self.a_src == SRC_STEP) as u64) << 3)
            | (((self.b_src == SRC_IMM) as u64) << 4)
            | (((self.b_src == SRC_MEM) as u64) << 5)
            | ((self.is_external_op as u64) << 6)
            | ((self.store_ra as u64) << 7)
            | (((self.store == STORE_MEM) as u64) << 8)
            | (((self.store == STORE_IND) as u64) << 9)
            | ((self.set_pc as u64) << 10)
            | ((self.m32 as u64) << 11)
            | (((self.b_src == SRC_IND) as u64) << 12);

        flags
    }
}
