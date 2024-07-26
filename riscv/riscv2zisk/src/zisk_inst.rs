use crate::{source_to_str, store_to_str};

/// ZisK instruction defined as a binary operation with 2 results: op(a, b) -> (c, flag)
/// a and b are loaded from the respective sources specified in the instruction
/// c is stored according to the destination specified in the instruction
/// flag can only be 0 or 1
pub struct ZiskInst {
    pub paddr: u64,
    pub store_ra: u64,
    pub store_use_sp: u64,
    pub store: u64,
    pub store_offset: i64,
    pub set_pc: u64,
    pub set_sp: u64,
    pub ind_width: u64,
    pub inc_sp: u64,
    pub end: u64,
    pub a_src: u64,
    pub a_use_sp_imm1: u64,
    pub a_offset_imm0: u64,
    pub b_src: u64,
    pub b_use_sp_imm1: u64,
    pub b_offset_imm0: u64,
    pub jmp_offset1: i64,
    pub jmp_offset2: i64,
    pub is_external_op: u64,
    pub op: u8,
    pub op_str: &'static str,
    pub verbose: String,
}

/// ZisK instruction implementation
impl ZiskInst {
    /// Creates a human-readable string containing the ZisK instruction fields that are not zero
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        if self.paddr != 0 {
            s += &(" paddr=".to_string() + &self.paddr.to_string());
        }
        if self.store_ra != 0 {
            s += &(" store_ra=".to_string() + &self.store_ra.to_string());
        }
        if self.store_use_sp != 0 {
            s += &(" store_use_sp=".to_string() + &self.store_use_sp.to_string());
        }
        if self.store != 0 {
            s += &format!(" store={}={}", self.store, store_to_str(self.store));
        }
        if self.store_offset != 0 {
            s += &(" store_offset=".to_string() + &self.store_offset.to_string());
        }
        if self.set_pc != 0 {
            s += &(" set_pc=".to_string() + &self.set_pc.to_string());
        }
        if self.set_sp != 0 {
            s += &(" set_sp=".to_string() + &self.set_sp.to_string());
        }
        if self.ind_width != 0 {
            s += &(" ind_width=".to_string() + &self.ind_width.to_string());
        }
        if self.inc_sp != 0 {
            s += &(" inc_sp=".to_string() + &self.inc_sp.to_string());
        }
        if self.end != 0 {
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
        if self.is_external_op != 0 {
            s += &(" is_external_op=".to_string() + &self.is_external_op.to_string());
        }
        {
            s += &(" op=".to_string() + &self.op.to_string() + "=" + self.op_str);
        }
        if !self.verbose.is_empty() {
            s += &(" verbose=".to_string() + &self.verbose);
        }
        s
    }
}
