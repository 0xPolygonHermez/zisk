use crate::{MemAlignResponse, MEM_BYTES};
use std::fmt;
use zisk_core::ZiskRequiredMemory;

fn format_u64_hex(value: u64) -> String {
    let hex_str = format!("{:016x}", value);
    hex_str
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("_")
}

const MAX_MEM_STEP_OFFSET: u64 = 2;
const MAX_MEM_OPS_PER_MAIN_STEP: u64 = (MAX_MEM_STEP_OFFSET + 1) * 2;

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

impl MemInput {
    pub fn new(addr: u32, is_write: bool, step: u64, value: u64, is_internal: bool) -> Self {
        MemInput { addr, is_write, step, value, is_internal }
    }
    pub fn from(mem_op: &ZiskRequiredMemory) -> Self {
        match mem_op {
            ZiskRequiredMemory::Basic { step, value, address, is_write, width, step_offset } => {
                debug_assert_eq!(*width, MEM_BYTES as u8);
                MemInput {
                    addr: address >> 3,
                    is_write: *is_write,
                    is_internal: false,
                    step: MemHelpers::main_step_to_address_step(*step, *step_offset),
                    value: *value,
                }
            }
            ZiskRequiredMemory::Extended { values: _, address: _ } => {
                panic!("MemInput::from() called with an extended instance");
            }
        }
    }
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
        1 + MAX_MEM_OPS_PER_MAIN_STEP * step + 2 * step_offset as u64
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

pub fn mem_align_call(
    mem_op: &ZiskRequiredMemory,
    mem_values: [u64; 2],
    phase: u8,
) -> MemAlignResponse {
    match mem_op {
        ZiskRequiredMemory::Basic { step, value, address, is_write, width, step_offset: _ } => {
            // DEBUG: only for testing
            let offset = (*address & 0x7) * 8;
            let width = (*width as u64) * 8;
            let double_address = (offset + width as u32) > 64;
            let mem_value = mem_values[phase as usize];
            let mask = 0xFFFF_FFFF_FFFF_FFFFu64 >> (64 - width);
            if *is_write {
                if phase == 0 {
                    MemAlignResponse {
                        more_addr: double_address,
                        step: step + 1,
                        value: Some(
                            (mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 ^ (mask << offset))) |
                                ((value & mask) << offset),
                        ),
                    }
                } else {
                    MemAlignResponse {
                        more_addr: false,
                        step: step + 1,
                        value: Some(
                            (mem_value &
                                (0xFFFF_FFFF_FFFF_FFFFu64 << (offset + width as u32 - 64))) |
                                ((value & mask) >> (128 - (offset + width as u32))),
                        ),
                    }
                }
            } else {
                MemAlignResponse {
                    more_addr: double_address && phase == 0,
                    step: step + 1,
                    value: None,
                }
            }
        }
        ZiskRequiredMemory::Extended { values: _, address: _ } => {
            panic!("mem_align_call() called with an extended instance");
        }
    }
}
