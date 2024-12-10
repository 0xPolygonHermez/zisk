use crate::{
    MemAlignResponse, MAX_MEM_OPS_BY_MAIN_STEP, MAX_MEM_OPS_BY_STEP_OFFSET, MEM_STEP_BASE,
};
use std::fmt;
use zisk_core::ZiskRequiredMemory;

#[allow(dead_code)]
fn format_u64_hex(value: u64) -> String {
    let hex_str = format!("{:016x}", value);
    hex_str
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("_")
}

#[derive(Debug, Clone)]
pub struct MemAlignInput {
    pub addr: u32,
    pub is_write: bool,
    pub width: u8,
    pub step: u64,
    pub value: u64,
    pub mem_values: [u64; 2],
}

#[derive(Debug, Clone)]
pub struct MemInput {
    pub addr: u32,         // address in word native format means byte_address / MEM_BYTES
    pub is_write: bool,    // it's a write operation
    pub is_internal: bool, // internal operation, don't send this operation to bus
    pub step: u64,         // mem_step = f(main_step, main_step_offset)
    pub value: u64,        // value to read or write
}

impl MemAlignInput {
    pub fn new(
        addr: u32,
        is_write: bool,
        width: u8,
        step: u64,
        value: u64,
        mem_values: [u64; 2],
    ) -> Self {
        MemAlignInput { addr, is_write, width, step, value, mem_values }
    }
    pub fn from(mem_external_op: &ZiskRequiredMemory, mem_values: &[u64; 2]) -> Self {
        match mem_external_op {
            ZiskRequiredMemory::Basic { step, value, address, is_write, width, step_offset } => {
                MemAlignInput {
                    addr: *address,
                    is_write: *is_write,
                    step: MemHelpers::main_step_to_address_step(*step, *step_offset),
                    width: *width,
                    value: *value,
                    mem_values: [mem_values[0], mem_values[1]],
                }
            }
            ZiskRequiredMemory::Extended { values: _, address: _ } => {
                panic!("MemAlignInput::from() called with extended instance")
            }
        }
    }
}

pub struct MemHelpers {}

impl MemHelpers {
    pub fn main_step_to_address_step(step: u64, step_offset: u8) -> u64 {
        MEM_STEP_BASE +
            MAX_MEM_OPS_BY_MAIN_STEP * step +
            MAX_MEM_OPS_BY_STEP_OFFSET * step_offset as u64
    }
}

impl fmt::Debug for MemAlignResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "more:{0} step:{1} value:{2:016X}({2:})",
            self.more_addr,
            self.step,
            self.value.unwrap_or(0)
        )
    }
}
