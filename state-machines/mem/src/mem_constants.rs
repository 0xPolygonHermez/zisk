pub const MEM_BYTES_BITS: u32 = 3;
pub const MEM_BYTES: u32 = 1 << MEM_BYTES_BITS;
pub const MEM_ADDR_ALIGN_MASK: u32 = MEM_BYTES - 1;
pub const MEM_ADDR_MASK: u32 = 0xFFFF_FFF8;

pub const MEM_STEP_BASE: u64 = 1;
pub const MAX_MEM_STEP_OFFSET: u64 = 2;
pub const MEM_STEPS_BY_MAIN_STEP: u64 = 4;

pub const MEMORY_LOAD_OP: u8 = 1;
pub const MEMORY_STORE_OP: u8 = 2;

pub const MEM_REGS_MASK: u32 = 0xFFFF_FF00;
pub const MEM_REGS_ADDR: u32 = 0xA000_0000;

pub const MAX_MAIN_STEP: u64 = 0x1FFF_FFFF_FFFF_FFFF;

pub const MAX_MEM_ADDR: u64 = 0xFFFF_FFFF;

pub const MEMORY_MAX_DIFF: u64 = 1 << 24;
pub const STEP_MEMORY_MAX_DIFF: u64 = MEMORY_MAX_DIFF - 1;

pub const CHUNK_SIZE: usize = 1 << 18;
pub const CHUNK_SIZE_STEPS: usize = CHUNK_SIZE * MEM_STEPS_BY_MAIN_STEP as usize;
pub const CHUNK_MAX_DISTANCE: usize = STEP_MEMORY_MAX_DIFF as usize / CHUNK_SIZE_STEPS;
pub const STEP_MEMORY_LIMIT_TO_VERIFY: usize = (CHUNK_MAX_DISTANCE - 2) * CHUNK_SIZE_STEPS;
