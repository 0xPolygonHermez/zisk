mod pil_helpers;

pub use pil_helpers::*;

//TODO To be removed when ready in ZISK_PIL
pub const ARITH32_AIR_IDS: &[usize] = &[4, 5];
pub const ARITH64_AIR_IDS: &[usize] = &[6];
pub const ARITH3264_AIR_IDS: &[usize] = &[7];
pub const MEM_AIRGROUP_ID: usize = 105;
pub const MEM_ALIGN_AIR_IDS: &[usize] = &[1];
pub const MEM_UNALIGNED_AIR_IDS: &[usize] = &[2, 3];
pub const QUICKOPS_AIRGROUP_ID: usize = 102;
pub const QUICKOPS_AIR_IDS: &[usize] = &[10];
