//! Address-space layout and register assignment for the wasm32 → Zisk lowering.
//!
//! Everything downstream of the produced `ZiskRom` is architecture-neutral, so the wasm machine
//! lives entirely inside the Zisk memory map defined in [`crate::mem`].  The constants here carve
//! the general-purpose RW region into the areas the wasm runtime needs (globals, indirect-call
//! table, control cells, linear memory and the call/operand stack), and pick which of the 32 Zisk
//! registers play which role.

use crate::{AVAILABLE_MEM_ADDR, FLOAT_LIB_RAM_ADDR};

// ---------------------------------------------------------------------------
// Register assignment (indices into the 32-register Zisk register file)
// ---------------------------------------------------------------------------

/// Return-address register, written by `store_pc` on a call (RISC-V `ra` convention).
pub const REG_RA: u64 = 1;
/// Frame pointer: the only pointer register live across wasm instruction boundaries.
pub const REG_FP: u64 = 2;
/// Scratch temporaries, dead across wasm instruction boundaries.
pub const REG_T0: u64 = 5;
pub const REG_T1: u64 = 6;
pub const REG_T2: u64 = 7;
pub const REG_T3: u64 = 8;
/// Function return value register.
pub const REG_RET: u64 = 10;

// ---------------------------------------------------------------------------
// Stack frame layout (grows downward from REG_FP)
// ---------------------------------------------------------------------------

/// Bytes reserved at the top of every frame for the header: `[FP-8]` = saved return PC,
/// `[FP-16]` = saved caller FP, `[FP-24]` = padding (keeps locals 8-byte aligned and leaves room).
pub const FRAME_HEADER_BYTES: i64 = 24;
/// Offset (from FP) of the saved return PC slot.
pub const FRAME_RET_PC_OFF: i64 = -8;
/// Offset (from FP) of the saved caller FP slot.
pub const FRAME_CALLER_FP_OFF: i64 = -16;

/// Offset (from FP) of wasm local `k` (locals include incoming parameters).
#[inline]
pub fn local_offset(k: u32) -> i64 {
    FRAME_HEADER_BYTES.wrapping_neg() - 8 * (k as i64)
}

/// Offset (from FP) of operand-stack slot at depth `d`, given the function's local count.
/// Operand slots sit immediately below the locals.
#[inline]
pub fn operand_offset(num_locals: u32, depth: u32) -> i64 {
    FRAME_HEADER_BYTES.wrapping_neg() - 8 * (num_locals as i64) - 8 * (depth as i64 + 1)
}

/// Total bytes a frame occupies: header + locals + the worst-case operand stack height.
#[inline]
pub fn frame_size(num_locals: u32, max_stack_depth: u32) -> i64 {
    FRAME_HEADER_BYTES + 8 * (num_locals as i64) + 8 * (max_stack_depth as i64)
}

// ---------------------------------------------------------------------------
// Memory regions (all inside the Zisk general-purpose RW window)
// ---------------------------------------------------------------------------

/// Base of wasm globals, 8 bytes each.
pub const WASM_GLOBALS_ADDR: u64 = AVAILABLE_MEM_ADDR; // 0xa0030000
/// Maximum number of globals (keeps globals well clear of the table area).
pub const WASM_MAX_GLOBALS: u64 = 0x2000; // 8192 globals -> 64 KiB

/// Base of the indirect-call table, 16 bytes per entry: `{ zisk_pc: u64, type_index: u64 }`.
pub const WASM_TABLE_ADDR: u64 = AVAILABLE_MEM_ADDR + 0x10000; // 0xa0040000
pub const WASM_TABLE_ENTRY_BYTES: u64 = 16;

/// Control cells used by the runtime.
pub const WASM_CTRL_ADDR: u64 = AVAILABLE_MEM_ADDR + 0x20000; // 0xa0050000
/// Current linear-memory size, in 64 KiB pages.
pub const WASM_MEM_PAGES_ADDR: u64 = WASM_CTRL_ADDR;
/// Read cursor into the input region, used by `fd_read` on stdin.
pub const WASM_STDIN_POS_ADDR: u64 = WASM_CTRL_ADDR + 8;
/// Number of bytes written to the public output so far, used by `fd_write` on stdout.
pub const WASM_STDOUT_LEN_ADDR: u64 = WASM_CTRL_ADDR + 16;

/// Base of wasm linear memory: wasm address 0 maps here.
pub const WASM_MEM_BASE: u64 = 0xa100_0000;
/// Page size, per the wasm spec (64 KiB).
pub const WASM_PAGE_SIZE: u64 = 0x10000;
/// Upper bound for linear memory, leaving a margin below the call stack.
pub const WASM_MEM_LIMIT: u64 = 0xbf00_0000;
/// Maximum number of linear-memory pages the runtime supports.
pub const WASM_MAX_PAGES: u64 = (WASM_MEM_LIMIT - WASM_MEM_BASE) / WASM_PAGE_SIZE;

/// Top of the call/operand stack; frames grow downward from here.  Kept below the float library
/// RAM region so the two never collide (wasm never touches the float lib, but we stay clear).
pub const WASM_STACK_TOP: u64 = FLOAT_LIB_RAM_ADDR - 0x10000; // 0xbffe0000

/// Absolute address of global `i`.
#[inline]
pub fn global_addr(i: u32) -> u64 {
    WASM_GLOBALS_ADDR + 8 * i as u64
}

/// Absolute address of indirect-call table entry `i`.
#[inline]
pub fn table_entry_addr(i: u32) -> u64 {
    WASM_TABLE_ADDR + WASM_TABLE_ENTRY_BYTES * i as u64
}
