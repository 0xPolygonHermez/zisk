use zisk_core::{MAX_ADDR, RAM_ADDR, RAM_SIZE};

/// Word addresses (byte address / 8) fit in a `u32`: the address space is 33 bits wide.
pub const RAM_W_ADDR_INIT: u32 = (RAM_ADDR >> MEM_BYTES_BITS) as u32;
pub const RAM_W_ADDR_END: u32 = ((RAM_ADDR + RAM_SIZE - 1) >> MEM_BYTES_BITS) as u32;

const _: () = {
    assert!((RAM_ADDR + RAM_SIZE - 1) <= MAX_ADDR, "RAM memory exceeds the addressable range");
    assert!((MAX_ADDR >> MEM_BYTES_BITS) <= u32::MAX as u64, "word addresses must fit in a u32");
};

pub const MEM_BYTES_BITS: u32 = 3;
pub const MEM_BYTES: u32 = 1 << MEM_BYTES_BITS;
pub const MEM_ADDR_ALIGN_MASK: u64 = (MEM_BYTES - 1) as u64;
pub const MEM_ADDR_MASK: u64 = !MEM_ADDR_ALIGN_MASK;

pub const MEM_STEP_BASE: u64 = 1;
pub const MAX_MEM_STEP_OFFSET: u64 = 2;
pub const MEM_STEPS_BY_MAIN_STEP_BITS: u64 = 2;
pub const MEM_STEPS_BY_MAIN_STEP: u64 = 1 << MEM_STEPS_BY_MAIN_STEP_BITS;

pub const MEMORY_LOAD_OP: u8 = 1;
pub const MEMORY_STORE_OP: u8 = 2;
pub const MEMORY_ROM_INIT_OP: u8 = 4;

pub const MEMORY_INIT_STEP: u64 = 3;

pub const MEM_REGS_MAX_DIFF: u64 = (1 << 24) - 1;
pub const MAX_MAIN_STEP: u64 = 0x1FFF_FFFF_FFFF_FFFF;

pub const MAX_MEM_ADDR: u64 = MAX_ADDR;

pub const SEGMENT_ADDR_MAX_DISTANCE: usize = 1 << 24;
pub const SEGMENT_ADDR_MAX_RANGE: usize = SEGMENT_ADDR_MAX_DISTANCE - 1;
pub const SEGMENT_LARGE_ADDR_C_MAX_RANGE: usize = (1 << 16) - 1;
