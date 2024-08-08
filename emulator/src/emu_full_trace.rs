#[derive(Default, Debug, Clone)]
pub struct EmuFullTraceStep<F> {
    pub a: F,
    pub b: F,
    pub c: F,
    pub last_c: F,
    pub flag: F,
    pub pc: F,
    pub a_src_imm: F,
    pub a_src_mem: F,
    pub a_offset_imm0: F,
    pub sp: F,
    pub a_src_sp: F,
    pub a_use_sp_imm1: F,
    pub a_src_step: F,
    pub b_src_imm: F,
    pub b_src_mem: F,
    pub b_offset_imm0: F,
    pub b_use_sp_imm1: F,
    pub b_src_ind: F,
    pub ind_width: F,
    pub is_external_op: F,
    pub op: F,
    pub store_ra: F,
    pub store_mem: F,
    pub store_ind: F,
    pub store_offset: F,
    pub set_pc: F,
    pub store_use_sp: F,
    pub set_sp: F,
    pub inc_sp: F,
    pub jmp_offset1: F,
    pub jmp_offset2: F,
}

#[derive(Default, Debug, Clone)]
pub struct EmuFullTrace<F> {
    pub steps: Vec<EmuFullTraceStep<F>>,
}
