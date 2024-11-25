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
    pub address: u32,
    pub is_write: bool,
    pub width: u8,
    pub step: u64,
    pub value: u64,
    pub mem_values: [u64; 2],
}

#[derive(Debug, Clone)]
pub struct MemInput {
    pub address: u32,
    pub is_write: bool,
    pub step: u64,
    pub value: u64,
}

impl MemInput {
    pub fn new(address: u32, is_write: bool, step: u64, value: u64) -> Self {
        MemInput { address, is_write, step, value }
    }
    pub fn from(mem_op: &ZiskRequiredMemory) -> Self {
        // debug_assert_eq!(mem_op.width, MEM_BYTES as u8);
        MemInput {
            address: mem_op.address,
            is_write: mem_op.is_write,
            step: MemHelpers::main_step_to_address_step(mem_op.step, mem_op.step_offset),
            value: mem_op.value,
        }
    }
}

impl MemAlignInput {
    pub fn new(
        address: u32,
        is_write: bool,
        width: u8,
        step: u64,
        value: u64,
        mem_values: [u64; 2],
    ) -> Self {
        MemAlignInput { address, is_write, width, step, value, mem_values }
    }
    pub fn from(mem_op: &MemInput, width: u8, mem_values: &[u64; 2]) -> Self {
        MemAlignInput {
            address: mem_op.address,
            is_write: mem_op.is_write,
            step: mem_op.step,
            width,
            value: mem_op.value,
            mem_values: [mem_values[0], mem_values[1]],
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
            self.more_address,
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
    // DEBUG: only for testing
    let offset = (mem_op.address & 0x7) * 8;
    let width = (mem_op.width as u64) * 8;
    let double_address = (offset + width as u32) > 64;
    let mem_value = mem_values[phase as usize];
    let mask = 0xFFFF_FFFF_FFFF_FFFFu64 >> (64 - width);
    if mem_op.is_write {
        if phase == 0 {
            MemAlignResponse {
                more_address: double_address,
                step: mem_op.step + 1,
                value: Some(
                    (mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 ^ (mask << offset))) |
                        ((mem_op.value & mask) << offset),
                ),
            }
        } else {
            MemAlignResponse {
                more_address: false,
                step: mem_op.step + 1,
                value: Some(
                    (mem_value & (0xFFFF_FFFF_FFFF_FFFFu64 << (offset + width as u32 - 64))) |
                        ((mem_op.value & mask) >> (128 - (offset + width as u32))),
                ),
            }
        }
    } else {
        MemAlignResponse {
            more_address: double_address && phase == 0,
            step: mem_op.step + 1,
            value: None,
        }
    }
}
