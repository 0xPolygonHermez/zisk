use crate::{
    MemAlignResponse, MEMORY_LOAD_OP, MEMORY_STORE_OP, MEM_ADDR_ALIGN_MASK, MEM_BYTES_BITS,
    MEM_STEPS_BY_MAIN_STEP, MEM_STEP_BASE, RAM_W_ADDR_INIT, STEP_MEMORY_MAX_DIFF,
};
use std::fmt;
use zisk_core::RAM_ADDR;
pub struct MemHelpers {}

impl MemHelpers {
    pub fn main_step_to_mem_step(step: u64, slot: u8) -> u64 {
        MEM_STEP_BASE + MEM_STEPS_BY_MAIN_STEP * step + slot as u64
    }
    pub fn main_step_to_precompiled_mem_step(step: u64, is_write: bool) -> u64 {
        MEM_STEP_BASE + MEM_STEPS_BY_MAIN_STEP * step + if is_write { 3 } else { 2 }
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
        if diff > STEP_MEMORY_MAX_DIFF {
            (diff - 1) / STEP_MEMORY_MAX_DIFF
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

    #[inline(always)]
    pub fn main_step_to_special_mem_step(main_step: u64) -> u64 {
        if main_step == 0 {
            0
        } else {
            Self::main_step_to_mem_step(main_step, 3)
        }
    }
    #[inline(always)]
    pub fn mem_step_to_slot(mem_step: u64) -> u8 {
        ((mem_step - MEM_STEP_BASE) % MEM_STEPS_BY_MAIN_STEP) as u8
    }
    #[inline(always)]
    pub fn mem_step_to_row(mem_step: u64) -> usize {
        ((mem_step - MEM_STEP_BASE) / MEM_STEPS_BY_MAIN_STEP) as usize
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
            return [lo_write, read_values[1]];
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
    /* struct MemHelpers {}

    const MEMORY_LOAD_OP: u64 = 1;
    const MEMORY_STORE_OP: u64 = 2;

    const MEM_STEP_BASE: u64 = 1;
    const MAX_MEM_OPS_BY_MAIN_STEP: u64 = 4;

    impl MemHelpers {
        // function mem_load(expr addr, expr step, expr step_offset = 0, expr bytes = 8, expr value[]) {
        // function mem_store(expr addr, expr step, expr step_offset = 0, expr bytes = 8, expr value[])
        // {*/
    pub fn mem_load(
        addr: u32,
        step: u64,
        step_offset: u8,
        bytes: u8,
        mem_values: [u64; 2],
    ) -> [u64; 7] {
        [
            MEMORY_LOAD_OP as u64,
            addr as u64,
            Self::main_step_to_mem_step(step, step_offset),
            bytes as u64,
            mem_values[0],
            mem_values[1],
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
            MEMORY_STORE_OP as u64,
            addr as u64,
            Self::main_step_to_mem_step(step, step_offset),
            bytes as u64,
            mem_values[0],
            mem_values[1],
            value,
        ]
    }
    /*#[inline(always)]
    pub fn main_step_to_mem_step(step: u64, step_offset: u8) -> u64 {
        MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + step_offset as u64
    }*/
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
