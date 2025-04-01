//! This module contains constant definitions used by other modules and crates.

pub const DEFAULT_MAX_STEPS: u64 = 0xffffffff;
pub const DEFAULT_MAX_STEPS_STR: &str = "4294967295"; // 2^32 - 1

/// Power of 2 constant definitions, named P2_n, equivalent to 2 to the power of n, in u64 format
pub const P2_0: u64 = 0x1;
pub const P2_1: u64 = 0x2;
pub const P2_2: u64 = 0x4;
pub const P2_3: u64 = 0x8;
pub const P2_4: u64 = 0x10;
pub const P2_5: u64 = 0x20;
pub const P2_6: u64 = 0x40;
pub const P2_7: u64 = 0x80;
pub const P2_8: u64 = 0x100;
pub const P2_9: u64 = 0x200;
pub const P2_10: u64 = 0x400;
pub const P2_11: u64 = 0x800;
pub const P2_12: u64 = 0x1000;
pub const P2_13: u64 = 0x2000;
pub const P2_14: u64 = 0x4000;
pub const P2_15: u64 = 0x8000;
pub const P2_16: u64 = 0x10000;
pub const P2_17: u64 = 0x20000;
pub const P2_18: u64 = 0x40000;
pub const P2_19: u64 = 0x80000;
pub const P2_20: u64 = 0x100000;
pub const P2_21: u64 = 0x200000;
pub const P2_22: u64 = 0x400000;
pub const P2_23: u64 = 0x800000;
pub const P2_24: u64 = 0x1000000;
pub const P2_25: u64 = 0x2000000;
pub const P2_26: u64 = 0x4000000;
pub const P2_27: u64 = 0x8000000;
pub const P2_28: u64 = 0x10000000;
pub const P2_29: u64 = 0x20000000;
pub const P2_30: u64 = 0x40000000;
pub const P2_31: u64 = 0x80000000;
pub const P2_32: u64 = 0x100000000;

/// Constant values used in operation functions and state machine executors
pub const M3: u64 = 0x7;
pub const M8: u64 = 0xFF;
pub const M16: u64 = 0xFFFF;
pub const M32: u64 = 0xFFFFFFFF;
pub const M64: u64 = 0xFFFFFFFFFFFFFFFF;
