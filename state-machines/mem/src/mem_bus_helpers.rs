use crate::{
    MAX_MEM_OPS_BY_MAIN_STEP, MAX_MEM_OPS_BY_STEP_OFFSET, MEMORY_LOAD_OP, MEMORY_STORE_OP,
    MEM_STEP_BASE,
};

pub struct MemBusHelpers {}

impl MemBusHelpers {
    // function mem_load(expr addr, expr step, expr step_offset = 0, expr bytes = 8, expr value[]) {
    // function mem_store(expr addr, expr step, expr step_offset = 0, expr bytes = 8, expr value[])
    // {
    pub fn mem_load(
        addr: u32,
        step: u64,
        step_offset: u8,
        bytes: u8,
        mem_values: [u64; 2],
    ) -> [u64; 7] {
        [
            MEMORY_LOAD_OP,
            addr as u64,
            MEM_STEP_BASE +
                MAX_MEM_OPS_BY_MAIN_STEP * step +
                MAX_MEM_OPS_BY_STEP_OFFSET * step_offset as u64,
            bytes as u64,
            mem_values[0] as u64,
            mem_values[1] as u64,
            0,
        ]
    }
    pub fn mem_write(
        addr: u32,
        step: u64,
        step_offset: u8,
        bytes: u8,
        value: u64,
        mem_values: [u64; 2],
    ) -> [u64; 7] {
        [
            MEMORY_STORE_OP,
            addr as u64,
            MEM_STEP_BASE +
                MAX_MEM_OPS_BY_MAIN_STEP * step +
                MAX_MEM_OPS_BY_STEP_OFFSET * step_offset as u64,
            bytes as u64,
            mem_values[0] as u64,
            mem_values[1] as u64,
            value,
        ]
    }
}
