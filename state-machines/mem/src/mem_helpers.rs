use crate::{
    MemAlignResponse, MAX_MEM_OPS_BY_MAIN_STEP, MAX_MEM_OPS_BY_STEP_OFFSET, MEMORY_MAX_DIFF,
    MEMORY_STORE_OP, MEM_ADDR_ALIGN_MASK, MEM_BYTES_BITS, MEM_STEP_BASE, RAM_W_ADDR_INIT,
};
use std::fmt;
use zisk_core::{ZiskRequiredMemory, RAM_ADDR};

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
    pub addr: u32,      // address in word native format means byte_address / MEM_BYTES
    pub is_write: bool, // it's a write operation
    pub step: u64,      // mem_step = f(main_step, main_step_offset)
    pub value: u64,     // value to read or write
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
    pub fn is_aligned(addr: u32, width: u8) -> bool {
        (addr & MEM_ADDR_ALIGN_MASK) == 0 && width == 8
    }
    pub fn get_addr_w(addr: u32) -> u32 {
        addr >> MEM_BYTES_BITS
    }
    pub fn get_addr(addr_w: u32) -> u32 {
        addr_w << MEM_BYTES_BITS
    }
    #[inline(always)]
    pub fn get_read_step(step: u64) -> u64 {
        step
    }
    #[inline(always)]
    pub fn get_write_step(step: u64) -> u64 {
        step + 1
    }
    #[inline(always)]
    pub fn is_double(addr: u32, bytes: u8) -> bool {
        (addr & MEM_ADDR_ALIGN_MASK) + bytes as u32 > 8
    }
    #[inline(always)]
    pub fn is_write(op: u8) -> bool {
        op == MEMORY_STORE_OP
    }
    #[inline(always)]
    pub fn get_byte_offset(addr: u32) -> u8 {
        (addr & MEM_ADDR_ALIGN_MASK) as u8
    }
    #[inline(always)]
    pub fn step_extra_reads_enabled(addr_w: u32) -> bool {
        addr_w as u64 >= RAM_ADDR
    }
    #[inline(always)]
    pub fn get_extra_internal_reads(previous_step: u64, step: u64) -> u64 {
        let diff = step - previous_step;
        if diff > MEMORY_MAX_DIFF {
            (diff - 1) / MEMORY_MAX_DIFF
        } else {
            0
        }
    }
    #[inline(always)]
    pub fn get_extra_internal_reads_by_addr(addr_w: u32, previous_step: u64, step: u64) -> u64 {
        if Self::step_extra_reads_enabled(addr_w) {
            Self::get_extra_internal_reads(previous_step, step)
        } else {
            0
        }
    }

    #[cfg(target_endian = "big")]
    compile_error!("This code requires a little-endian machine.");
    pub fn get_write_values(addr: u32, bytes: u8, value: u64, read_values: [u64; 2]) -> [u64; 2] {
        let is_double = Self::is_double(addr, bytes);
        let offset = Self::get_byte_offset(addr) * 8;
        let value = match bytes {
            1 => value & 0xFF,
            2 => value & 0xFFFF,
            4 => value & 0xFFFF_FFFF,
            8 => value,
            _ => panic!("Invalid bytes value"),
        };
        let byte_mask = match bytes {
            1 => 0xFFu64,
            2 => 0xFFFFu64,
            4 => 0xFFFF_FFFFu64,
            8 => 0xFFFF_FFFF_FFFF_FFFFu64,
            _ => panic!("Invalid bytes value"),
        };

        let lo_mask = !(byte_mask << offset);
        let lo_write = (lo_mask & read_values[0]) | (value << offset);
        if !is_double {
            return [lo_write, read_values[1]]
        }

        let hi_mask = !(byte_mask >> (64 - offset));
        let hi_write = (hi_mask & read_values[1]) | (value >> (64 - offset));

        [lo_write, hi_write]
    }
    #[cfg(target_endian = "big")]
    compile_error!("This code requires a little-endian machine.");
    pub fn get_read_value(addr: u32, bytes: u8, read_values: [u64; 2]) -> u64 {
        let is_double = Self::is_double(addr, bytes);
        let offset = Self::get_byte_offset(addr) * 8;
        let mut value = read_values[0] >> offset;
        if is_double {
            value |= (read_values[1] >> offset) << (64 - offset);
        }
        match bytes {
            1 => value & 0xFF,
            2 => value & 0xFFFF,
            4 => value & 0xFFFF_FFFF,
            8 => value,
            _ => panic!("Invalid bytes value"),
        }
    }
    pub fn register_to_addr(register: u8) -> u32 {
        ((RAM_ADDR + register as u64) * 8) as u32
    }
    pub fn register_to_addr_w(register: u8) -> u32 {
        RAM_W_ADDR_INIT + register as u32
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
