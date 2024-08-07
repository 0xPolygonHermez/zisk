#[derive(Default, Debug, Clone)]
pub struct EmuFullTraceStep {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub last_c: u64,
    pub flag: bool,
    pub pc: u64,
    pub a_src_imm: u64,
    pub a_src_mem: u64,
    pub a_offset_imm0: u64,
    pub sp: u64,
    pub a_src_sp: u64,
    pub a_use_sp_imm1: u64,
    pub a_src_step: u64,
    pub b_src_imm: u64,
    pub b_src_mem: u64,
    pub b_offset_imm0: u64,
    pub b_use_sp_imm1: u64,
    pub b_src_ind: u64,
    pub ind_width: u64,
    pub is_external_op: bool,
    pub op: u8,
    pub store_ra: bool,
    pub store_mem: u64,
    pub store_ind: u64,
    pub store_offset: i64,
    pub set_pc: bool,
    pub store_use_sp: bool,
    pub set_sp: bool,
    pub inc_sp: u64,
    pub jmp_offset1: i64,
    pub jmp_offset2: i64,
}

#[derive(Default, Debug, Clone)]
pub struct EmuFullTrace {
    pub steps: Vec<EmuFullTraceStep>,
}
