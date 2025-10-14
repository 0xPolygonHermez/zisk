//! Zisk registers
//!
//! # RISC-V registers memory mapping
//!
//! The 32 8-bytes RISC-V registers are mapped to RW memory starting at address SYS_ADDR.
//! They occupy 32x8=256 bytes of memory space.
//!
//! References:
//!     - https://riscv-non-isa.github.io/riscv-elf-psabi-doc/#_register_convention
//!     - https://github.com/riscv-non-isa/riscv-elf-psabi-doc
//!
//! | ABI name | X name  | Usage                                     |
//! |----------|---------|-------------------------------------------|
//! | REG_ZERO | REG_X0  | Read always as zero                       |
//! | REG_RA   | REG_X1  | Return address                            |
//! | REG_SP   | REG_X2  | Stack pointer                             |
//! | REG_GP   | REG_X3  | Global pointer                            |
//! | REG_TP   | REG_X4  | Thread pointer                            |
//! | REG_T0   | REG_X5  | Temporary register 0                      |
//! | REG_T1   | REG_X6  | Temporary register 1                      |
//! | REG_T2   | REG_X7  | Temporary register 2                      |
//! | REG_S0   | REG_X8  | Saved register 0 / frame pointer          |
//! | REG_S1   | REG_X9  | Saved register 1                          |
//! | REG_A0   | REG_X10 | Function argument 0 / return value 0      |
//! | REG_A1   | REG_X11 | Function argument 1 / return value 1      |
//! | REG_A2   | REG_X12 | Function argument 2                       |
//! | REG_A3   | REG_X13 | Function argument 3                       |
//! | REG_A4   | REG_X14 | Function argument 4                       |
//! | REG_A5   | REG_X15 | Function argument 5                       |
//! | REG_A6   | REG_X16 | Function argument 6                       |
//! | REG_A7   | REG_X17 | Function argument 7                       |
//! | REG_S2   | REG_X18 | Saved register 2                          |
//! | REG_S3   | REG_X19 | Saved register 3                          |
//! | REG_S4   | REG_X20 | Saved register 4                          |
//! | REG_S5   | REG_X21 | Saved register 5                          |
//! | REG_S6   | REG_X22 | Saved register 6                          |
//! | REG_S7   | REG_X23 | Saved register 7                          |
//! | REG_S8   | REG_X24 | Saved register 8                          |
//! | REG_S9   | REG_X25 | Saved register 9                          |
//! | REG_S10  | REG_X26 | Saved register 10                         |
//! | REG_S11  | REG_X27 | Saved register 11                         |
//! | REG_T3   | REG_X28 | Temporary register 3                      |
//! | REG_T4   | REG_X29 | Temporary register 4                      |
//! | REG_T5   | REG_X30 | Temporary register 5                      |
//! | REG_T6   | REG_X31 | Temporary register 6                      |

use crate::{FREG_FIRST, SYS_ADDR};

// Registers memory address definitions
pub const REG_FIRST: u64 = SYS_ADDR;

// These are the generic register names, i.e. REG_Xn.
pub const REG_X0: u64 = REG_FIRST;
pub const REG_X1: u64 = REG_FIRST + 8;
pub const REG_X2: u64 = REG_FIRST + 2_u64 * 8;
pub const REG_X3: u64 = REG_FIRST + 3_u64 * 8;
pub const REG_X4: u64 = REG_FIRST + 4_u64 * 8;
pub const REG_X5: u64 = REG_FIRST + 5_u64 * 8;
pub const REG_X6: u64 = REG_FIRST + 6_u64 * 8;
pub const REG_X7: u64 = REG_FIRST + 7_u64 * 8;
pub const REG_X8: u64 = REG_FIRST + 8_u64 * 8;
pub const REG_X9: u64 = REG_FIRST + 9_u64 * 8;
pub const REG_X10: u64 = REG_FIRST + 10_u64 * 8;
pub const REG_X11: u64 = REG_FIRST + 11_u64 * 8;
pub const REG_X12: u64 = REG_FIRST + 12_u64 * 8;
pub const REG_X13: u64 = REG_FIRST + 13_u64 * 8;
pub const REG_X14: u64 = REG_FIRST + 14_u64 * 8;
pub const REG_X15: u64 = REG_FIRST + 15_u64 * 8;
pub const REG_X16: u64 = REG_FIRST + 16_u64 * 8;
pub const REG_X17: u64 = REG_FIRST + 17_u64 * 8;
pub const REG_X18: u64 = REG_FIRST + 18_u64 * 8;
pub const REG_X19: u64 = REG_FIRST + 19_u64 * 8;
pub const REG_X20: u64 = REG_FIRST + 20_u64 * 8;
pub const REG_X21: u64 = REG_FIRST + 21_u64 * 8;
pub const REG_X22: u64 = REG_FIRST + 22_u64 * 8;
pub const REG_X23: u64 = REG_FIRST + 23_u64 * 8;
pub const REG_X24: u64 = REG_FIRST + 24_u64 * 8;
pub const REG_X25: u64 = REG_FIRST + 25_u64 * 8;
pub const REG_X26: u64 = REG_FIRST + 26_u64 * 8;
pub const REG_X27: u64 = REG_FIRST + 27_u64 * 8;
pub const REG_X28: u64 = REG_FIRST + 28_u64 * 8;
pub const REG_X29: u64 = REG_FIRST + 29_u64 * 8;
pub const REG_X30: u64 = REG_FIRST + 30_u64 * 8;
pub const REG_X31: u64 = REG_FIRST + 31_u64 * 8;

pub const REG_LAST: u64 = REG_X31;

// ABI register names.
pub const REG_ZERO: u64 = REG_X0;
pub const REG_RA: u64 = REG_X1; // Return address
pub const REG_SP: u64 = REG_X2; // Stack pointer
pub const REG_GP: u64 = REG_X3; // Global pointer
pub const REG_TP: u64 = REG_X4; // Thread pointer
pub const REG_T0: u64 = REG_X5; // Temporary register 0
pub const REG_T1: u64 = REG_X6; // Temporary register 1
pub const REG_T2: u64 = REG_X7; // Temporary register 2
pub const REG_S0: u64 = REG_X8; // Saved register 0 / frame pointer
pub const REG_S1: u64 = REG_X9; // Saved register 1
pub const REG_A0: u64 = REG_X10; // Function argument 0 / return value 0
pub const REG_A1: u64 = REG_X11; // Function argument 1 / return value 1
pub const REG_A2: u64 = REG_X12; // Function argument 2
pub const REG_A3: u64 = REG_X13; // Function argument 3
pub const REG_A4: u64 = REG_X14; // Function argument 4
pub const REG_A5: u64 = REG_X15; // Function argument 5
pub const REG_A6: u64 = REG_X16; // Function argument 6
pub const REG_A7: u64 = REG_X17; // Function argument 7
pub const REG_S2: u64 = REG_X18; // Saved register 2
pub const REG_S3: u64 = REG_X19; // Saved register 3
pub const REG_S4: u64 = REG_X20; // Saved register 4
pub const REG_S5: u64 = REG_X21; // Saved register 5
pub const REG_S6: u64 = REG_X22; // Saved register 6
pub const REG_S7: u64 = REG_X23; // Saved register 7
pub const REG_S8: u64 = REG_X24; // Saved register 8
pub const REG_S9: u64 = REG_X25; // Saved register 9
pub const REG_S10: u64 = REG_X26; // Saved register 10
pub const REG_S11: u64 = REG_X27; // Saved register 11
pub const REG_T3: u64 = REG_X28; // Temporary register 3
pub const REG_T4: u64 = REG_X29; // Temporary register 4
pub const REG_T5: u64 = REG_X30; // Temporary register 5
pub const REG_T6: u64 = REG_X31; // Temporary register 6

pub const REGS_IN_MAIN_FROM: usize = 1; // First non-zero register in main trace
pub const REGS_IN_MAIN_TO: usize = 31; // Last non-zero register in main trace
pub const REGS_IN_MAIN: usize = REGS_IN_MAIN_TO - REGS_IN_MAIN_FROM + 1;
pub const REGS_IN_MAIN_TOTAL_NUMBER: usize = 32; // Total number of registers in main, including the zero register

// Float registers memory address definitions
pub const FREG_F0: u64 = FREG_FIRST;
pub const FREG_F1: u64 = FREG_FIRST + 8;
pub const FREG_F2: u64 = FREG_FIRST + 2_u64 * 8;
pub const FREG_F3: u64 = FREG_FIRST + 3_u64 * 8;
pub const FREG_F4: u64 = FREG_FIRST + 4_u64 * 8;
pub const FREG_F5: u64 = FREG_FIRST + 5_u64 * 8;
pub const FREG_F6: u64 = FREG_FIRST + 6_u64 * 8;
pub const FREG_F7: u64 = FREG_FIRST + 7_u64 * 8;
pub const FREG_F8: u64 = FREG_FIRST + 8_u64 * 8;
pub const FREG_F9: u64 = FREG_FIRST + 9_u64 * 8;
pub const FREG_F10: u64 = FREG_FIRST + 10_u64 * 8;
pub const FREG_F11: u64 = FREG_FIRST + 11_u64 * 8;
pub const FREG_F12: u64 = FREG_FIRST + 12_u64 * 8;
pub const FREG_F13: u64 = FREG_FIRST + 13_u64 * 8;
pub const FREG_F14: u64 = FREG_FIRST + 14_u64 * 8;
pub const FREG_F15: u64 = FREG_FIRST + 15_u64 * 8;
pub const FREG_F16: u64 = FREG_FIRST + 16_u64 * 8;
pub const FREG_F17: u64 = FREG_FIRST + 17_u64 * 8;
pub const FREG_F18: u64 = FREG_FIRST + 18_u64 * 8;
pub const FREG_F19: u64 = FREG_FIRST + 19_u64 * 8;
pub const FREG_F20: u64 = FREG_FIRST + 20_u64 * 8;
pub const FREG_F21: u64 = FREG_FIRST + 21_u64 * 8;
pub const FREG_F22: u64 = FREG_FIRST + 22_u64 * 8;
pub const FREG_F23: u64 = FREG_FIRST + 23_u64 * 8;
pub const FREG_F24: u64 = FREG_FIRST + 24_u64 * 8;
pub const FREG_F25: u64 = FREG_FIRST + 25_u64 * 8;
pub const FREG_F26: u64 = FREG_FIRST + 26_u64 * 8;
pub const FREG_F27: u64 = FREG_FIRST + 27_u64 * 8;
pub const FREG_F28: u64 = FREG_FIRST + 28_u64 * 8;
pub const FREG_F29: u64 = FREG_FIRST + 29_u64 * 8;
pub const FREG_F30: u64 = FREG_FIRST + 30_u64 * 8;
pub const FREG_F31: u64 = FREG_FIRST + 31_u64 * 8;

// TODO: delete the _INVALID version after testing
pub const FREG_INST: u64 = FREG_FIRST + 33_u64 * 8; // Floating-point instruction register (finst)
pub const FREG_RA: u64 = FREG_FIRST + 34_u64 * 8; // Floating-point return address register (fra)

// RISC-V registers backup when calling float functions
pub const FREG_X0: u64 = FREG_FIRST + 35_u64 * 8; // Floating-point register x0
pub const FREG_X1: u64 = FREG_FIRST + 36_u64 * 8; // Floating-point register x1
pub const FREG_X2: u64 = FREG_FIRST + 37_u64 * 8; // Floating-point register x2
pub const FREG_X3: u64 = FREG_FIRST + 38_u64 * 8; // Floating-point register x3
pub const FREG_X4: u64 = FREG_FIRST + 39_u64 * 8; // Floating-point register x4
pub const FREG_X5: u64 = FREG_FIRST + 40_u64 * 8; // Floating-point register x5
pub const FREG_X6: u64 = FREG_FIRST + 41_u64 * 8; // Floating-point register x6
pub const FREG_X7: u64 = FREG_FIRST + 42_u64 * 8; // Floating-point register x7
pub const FREG_X8: u64 = FREG_FIRST + 43_u64 * 8; // Floating-point register x8
pub const FREG_X9: u64 = FREG_FIRST + 44_u64 * 8; // Floating-point register x9
pub const FREG_X10: u64 = FREG_FIRST + 45_u64 * 8; // Floating-point register x10
pub const FREG_X11: u64 = FREG_FIRST + 46_u64 * 8; // Floating-point register x11
pub const FREG_X12: u64 = FREG_FIRST + 47_u64 * 8; // Floating-point register x12
pub const FREG_X13: u64 = FREG_FIRST + 48_u64 * 8; // Floating-point register x13
pub const FREG_X14: u64 = FREG_FIRST + 49_u64 * 8; // Floating-point register x14
pub const FREG_X15: u64 = FREG_FIRST + 50_u64 * 8; // Floating-point register x15
pub const FREG_X16: u64 = FREG_FIRST + 51_u64 * 8; // Floating-point register x16
pub const FREG_X17: u64 = FREG_FIRST + 52_u64 * 8; // Floating-point register x17
pub const FREG_X18: u64 = FREG_FIRST + 53_u64 * 8; // Floating-point register x18
pub const FREG_X19: u64 = FREG_FIRST + 54_u64 * 8; // Floating-point register x19
pub const FREG_X20: u64 = FREG_FIRST + 55_u64 * 8; // Floating-point register x20
pub const FREG_X21: u64 = FREG_FIRST + 56_u64 * 8; // Floating-point register x21
pub const FREG_X22: u64 = FREG_FIRST + 57_u64 * 8; // Floating-point register x22
pub const FREG_X23: u64 = FREG_FIRST + 58_u64 * 8; // Floating-point register x23
pub const FREG_X24: u64 = FREG_FIRST + 59_u64 * 8; // Floating-point register x24
pub const FREG_X25: u64 = FREG_FIRST + 60_u64 * 8; // Floating-point register x25
pub const FREG_X26: u64 = FREG_FIRST + 61_u64 * 8; // Floating-point register x26
pub const FREG_X27: u64 = FREG_FIRST + 62_u64 * 8; // Floating-point register x27
pub const FREG_X28: u64 = FREG_FIRST + 63_u64 * 8; // Floating-point register x28
pub const FREG_X29: u64 = FREG_FIRST + 64_u64 * 8; // Floating-point register x29
pub const FREG_X30: u64 = FREG_FIRST + 65_u64 * 8; // Floating-point register x30
pub const FREG_X31: u64 = FREG_FIRST + 66_u64 * 8; // Floating-point register x31
