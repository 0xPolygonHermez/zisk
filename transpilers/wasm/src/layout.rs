//! Address-space layout and register assignment for the wasm32 → Zisk lowering.
//!
//! Everything downstream of the produced `ZiskRom` is architecture-neutral, so the wasm machine
//! lives entirely inside the Zisk memory map defined in [`zisk_core::mem`].  The constants here
//! carve the general-purpose RW region into the areas the wasm runtime needs (globals,
//! indirect-call table, control cells, linear memory and the call/operand stack), and pick which
//! of the 32 Zisk registers play which role.

use zisk_core::{FLOAT_LIB_RAM_ADDR, OUTPUT_ADDR, OUTPUT_MAX_SIZE, ZISKLIB_RAM_ADDR};

pub const WASM_PUBLIC_OUTPUT_BYTES: u64 = 32 * 8;

/// Base of the Zisk general-purpose RW region, i.e. the first address above the output region.
/// The wasm runtime areas below are carved out of it.
const GP_RAM_ADDR: u64 = OUTPUT_ADDR + OUTPUT_MAX_SIZE;

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
/// Holds the operand-stack value an operator just produced, so the next operator can use it
/// without a store and a load (see `Val::Acc` in the lowering).  Only ever live inside
/// straight-line code: every label, call and handler is preceded by a flush to the slots.  It
/// is the return register, so a call's result needs no move to get there, nor the value a
/// function returns.
pub const REG_ACC: u64 = REG_RET;
/// The low half of a 128-bit result (`i64.mul_wide_*`, `i64.add128`, `i64.sub128`) whose high
/// half is in `REG_ACC`.  Live only in straight-line code, like `REG_ACC`, so the float
/// library's use of `x32`..`x39` as scratch cannot reach it.
pub const REG_ACC2: u64 = 32;
/// One past the last valid absolute linear-memory address (`WASM_MEM_BASE + pages * 64 KiB`).
/// Live for the whole run: set by the entry routine and only ever changed by `memory.grow`; every
/// linear-memory access is bounds-checked against it.  WASI stubs and the float handler must
/// preserve it (the handler saves/restores every register).
pub const REG_MEM_END: u64 = 9;
/// Registers that hold a function's most used locals (see `LocalRegs` in the lowering), in the
/// order they are handed out.  The caller saves them around every call, so a callee, a stub or
/// a ZisK library routine may use them freely; the float handler restores all of them.  `x32`
/// and up are left out: the float library uses them as scratch and the handler does not
/// restore them.
pub const REG_LOCALS: [u64; 17] =
    [11, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 31, 3, 4];
/// Absolute linear-memory addresses made from a local, reused by later accesses through the
/// same local in straight-line code (see `Ptr` in the lowering).  Dead at every label and call.
pub const REG_PTRS: [u64; 2] = [12, 13];
/// `(width, register)`: the register holds `REG_MEM_END - width`, the last address an access of
/// that width may start at, so a load or store is checked with one compare.  Kept in step with
/// `REG_MEM_END` (see [`mem_limit_reg`]): WASI stubs and the float handler preserve them, and the
/// zkvm stubs recompute them after a ZisK library routine, for which they are RISC-V temporaries.
pub const REG_MEM_LIMITS: [(u64, u64); 4] = [(1, 27), (2, 28), (4, 29), (8, 30)];

/// The register holding `REG_MEM_END - width` for an access of `width` bytes.
pub fn mem_limit_reg(width: u64) -> u64 {
    REG_MEM_LIMITS
        .iter()
        .find(|&&(w, _)| w == width)
        .map(|&(_, reg)| reg)
        .unwrap_or_else(|| panic!("wasm: no bounds register for a {width}-byte access"))
}

// ---------------------------------------------------------------------------
// Stack frame layout (grows downward from REG_FP)
// ---------------------------------------------------------------------------

/// Bytes reserved at the top of every frame for the header: `[FP-8]` = saved return PC (unused
/// by a function that calls nothing: it returns through `REG_RA`), `[FP-16]` and `[FP-24]`
/// unused.  The caller moves FP down by its own frame size before a call and back up after
/// it, so a frame does not record its caller's FP.
pub const FRAME_HEADER_BYTES: i64 = 24;
/// Offset (from FP) of the saved return PC slot.
pub const FRAME_RET_PC_OFF: i64 = -8;

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
pub const WASM_GLOBALS_ADDR: u64 = GP_RAM_ADDR; // 0xa0430000
/// Capacity of the globals area, in globals (64 KiB).  Enforced at transpile time.
pub const WASM_MAX_GLOBALS: u64 = 0x2000;

/// Base of the indirect-call table, 16 bytes per entry: `{ zisk_pc: u64, type_index: u64 }`.
pub const WASM_TABLE_ADDR: u64 = WASM_GLOBALS_ADDR + 8 * WASM_MAX_GLOBALS; // 0xa0440000
pub const WASM_TABLE_ENTRY_BYTES: u64 = 16;
/// Capacity of the table area, in entries (1 MiB; a Go program declares ~12k).  Enforced at
/// transpile time against the declared table size, and at run time by `call_indirect`.
pub const WASM_MAX_TABLE_ENTRIES: u64 = 0x10000;

/// Control cells used by the runtime.
pub const WASM_CTRL_ADDR: u64 = WASM_TABLE_ADDR + WASM_TABLE_ENTRY_BYTES * WASM_MAX_TABLE_ENTRIES; // 0xa0540000
/// Current linear-memory size, in 64 KiB pages.
pub const WASM_MEM_PAGES_ADDR: u64 = WASM_CTRL_ADDR;
/// Read cursor into the input region, used by `fd_read` on stdin.
pub const WASM_STDIN_POS_ADDR: u64 = WASM_CTRL_ADDR + 8;
/// Number of bytes written to the public output so far, used by `fd_write` on stdout.
pub const WASM_STDOUT_LEN_ADDR: u64 = WASM_CTRL_ADDR + 16;
pub const WASM_CLOCK_ADDR: u64 = WASM_CTRL_ADDR + 24;
/// State of the deterministic generator behind `random_get` (see `wasi.rs`).
pub const WASM_RNG_STATE_ADDR: u64 = WASM_CTRL_ADDR + 32;
/// Non-zero once the insecure-randomness warning has been printed.
pub const WASM_RNG_WARNED_ADDR: u64 = WASM_CTRL_ADDR + 40;
/// Scratch parameter block for the precompiles that take their operands through a block of
/// pointers (see `precompiles.rs`): the guest's wasm-address block is translated here, so the
/// precompile dereferences absolute addresses.  Eight 8-byte slots; the largest block has five.
pub const WASM_PRECOMPILE_BLOCK_ADDR: u64 = WASM_CTRL_ADDR + 64;
pub const WASM_PRECOMPILE_BLOCK_SLOTS: u64 = 8;

/// Base of wasm linear memory: wasm address 0 maps here.
/// It sits 192 KiB lower than the 0xa100_0000 it started at, so that reserving the ZisK library
/// RAM above the call stack took that room from the gap after the control cells, not from the
/// memory or the stack.
pub const WASM_MEM_BASE: u64 = 0xa0fd_0000;
const _: () = assert!(WASM_MEM_BASE >= WASM_CTRL_ADDR + 0x10000);
/// Page size, per the wasm spec (64 KiB).
pub const WASM_PAGE_SIZE: u64 = 0x10000;
/// Upper bound for linear memory, where the call stack ends: 7,680 pages (480 MiB) above the base.
pub const WASM_MEM_LIMIT: u64 = 0xbefd_0000;
/// Maximum number of linear-memory pages the runtime supports.
pub const WASM_MAX_PAGES: u64 = (WASM_MEM_LIMIT - WASM_MEM_BASE) / WASM_PAGE_SIZE;

/// Top of the call/operand stack; frames grow downward from here.  Kept below the ZisK library
/// RAM region (the accelerator routines' scratch, see `zkvm.rs`), which sits just below the float
/// library RAM, so neither is ever overwritten by a frame.
pub const WASM_STACK_TOP: u64 = ZISKLIB_RAM_ADDR - 0x10000; // 0xbffb0000
const _: () = assert!(WASM_STACK_TOP < ZISKLIB_RAM_ADDR && ZISKLIB_RAM_ADDR < FLOAT_LIB_RAM_ADDR);
// 7,680 pages of memory and a 0xfe_0000-byte stack (about 2,000 frames), as before the ZisK
// library RAM was reserved.
const _: () = assert!(WASM_MAX_PAGES == 7680 && WASM_STACK_TOP - WASM_STACK_LIMIT == 0xfe_0000);
/// Lowest address a frame may occupy: the stack ends where linear memory can grow to.  Calls trap
/// (stack exhaustion) instead of placing a frame below it.
pub const WASM_STACK_LIMIT: u64 = WASM_MEM_LIMIT;

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
