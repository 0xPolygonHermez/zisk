// a and b registers source types
pub const SRC_C: u64 = 0;
pub const SRC_MEM: u64 = 1;
pub const SRC_IMM: u64 = 2;
pub const SRC_STEP: u64 = 3;
// #[cfg(feature = "sp")]
// pub const SRC_SP: u64 = 4;
pub const SRC_IND: u64 = 5;

// c register store destination types
pub const STORE_NONE: u64 = 0;
pub const STORE_MEM: u64 = 1;
pub const STORE_IND: u64 = 2;

/* Memory map:

  |--------------- ROM_ENTRY (0x1000)
  | (rom entry, calls program, ~BIOS)
  |---------------
        ...
  |--------------- ROM_ADDR (0x80000000)
  | (rom program)
  |--------------- INPUT_ADDR
  | (input data)
  |--------------- SYS_ADDR (= RAM_ADDR)
  | (sys = 32 registers)
  |--------------- OUTPUT_ADDR
  | (output data)
  |--------------- AVAILABLE_MEM_ADDR
  | (program memory)
  |---------------

*/
pub const ROM_ADDR: u64 = 0x80000000;
pub const ROM_ADDR_MAX: u64 = INPUT_ADDR - 1;
pub const INPUT_ADDR: u64 = 0x90000000;
pub const MAX_INPUT_SIZE: u64 = 0x10000000; // 256M,
pub const RAM_ADDR: u64 = 0xa0000000;
pub const RAM_SIZE: u64 = 0x10000000; // 256M
pub const SYS_ADDR: u64 = RAM_ADDR;
pub const SYS_SIZE: u64 = 0x10000;
pub const OUTPUT_ADDR: u64 = SYS_ADDR + SYS_SIZE;
pub const OUTPUT_MAX_SIZE: u64 = 0x10000; // 64K
pub const AVAILABLE_MEM_ADDR: u64 = OUTPUT_ADDR + OUTPUT_MAX_SIZE;
pub const AVAILABLE_MEM_SIZE: u64 = RAM_SIZE - OUTPUT_MAX_SIZE - SYS_SIZE;
pub const ROM_ENTRY: u64 = 0x1000;
pub const ARCH_ID_ZISK: u64 = 0xFFFEEEE;
pub const UART_ADDR: u64 = SYS_ADDR + 512;

// Powers of 2 definitions
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

// Registers definitions

pub const REG_FIRST: u64 = SYS_ADDR;

// The 32 registers are mapped to the first 32x8 bytes of system memory
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

// ABI register names
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
