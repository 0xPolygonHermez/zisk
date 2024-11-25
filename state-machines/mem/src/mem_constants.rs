pub const MEM_ADDR_MASK: u64 = 0xFFFF_FFFF_FFFF_FFF8;
pub const MEM_BYTES: u64 = 8;

pub const MAX_MEM_STEP_OFFSET: u64 = 2;
pub const MAX_MEM_OPS_PER_MAIN_STEP: u64 = (MAX_MEM_STEP_OFFSET + 1) * 2;

pub const MEM_STEP_BITS: u64 = 34; // with step_slot = 8 => 2GB steps (
pub const MEM_STEP_MASK: u64 = (1 << MEM_STEP_BITS) - 1; // 256 MB
pub const MEM_ADDR_BITS: u64 = 64 - MEM_STEP_BITS;

pub const MAX_MEM_STEP: u64 = (1 << MEM_STEP_BITS) - 1;
pub const MAX_MEM_ADDR: u64 = 0xFFFF_FFFF;
