//! Kinds of allocation mark (`SYSCALL_ALLOC_MARK_ID`).
//!
//! A mark is a no-op instruction a guest allocator emits to tell the emulator what it is doing,
//! so that `ziskemu --alloc-stats` can analyze the use of the allocator. It carries two values
//! (`a` and `b`) and one of these kinds. A call to the allocator is a sequence of marks:
//!
//! | call           | marks                                                              |
//! |----------------|--------------------------------------------------------------------|
//! | `alloc`        | `ALLOC(size, align)` ... `END(ptr)`                                |
//! | `alloc_zeroed` | `ALLOC_ZEROED(size, align)` ... `END(ptr)`                         |
//! | `dealloc`      | `LAYOUT(size, align)`, `DEALLOC(ptr)` ... `END`                    |
//! | `realloc`      | `LAYOUT(old_size, align)`, `REALLOC(ptr, new_size)` ... `END(ptr)` |
//!
//! What runs between the opening mark and `END` is charged to the allocator. `LAYOUT` is
//! optional (C `free` and `realloc` do not know it), and an alignment of 0 means "not stated".

/// Opens an allocation: `a` = size, `b` = alignment.
pub const ALLOC_MARK_ALLOC_ID: u8 = 1;
/// Opens a zeroed allocation: `a` = size, `b` = alignment.
pub const ALLOC_MARK_ALLOC_ZEROED_ID: u8 = 2;
/// Layout of the block the next `DEALLOC` or `REALLOC` works on: `a` = size, `b` = alignment.
pub const ALLOC_MARK_LAYOUT_ID: u8 = 3;
/// Opens a release: `a` = pointer.
pub const ALLOC_MARK_DEALLOC_ID: u8 = 4;
/// Opens a reallocation: `a` = pointer, `b` = new size.
pub const ALLOC_MARK_REALLOC_ID: u8 = 5;
/// Closes the call in progress: `a` = pointer it returns (ignored for a release).
pub const ALLOC_MARK_END_ID: u8 = 6;

// Allocator precompile (PROTOTYPE, see `precompiles/allocator`). Its operations travel through
// the same CSR as the marks, with kinds of their own, and are transpiled to a `copyb` too: the
// prover sees a no-op, and ziskemu runs the operation on the heap of the precompile. Only ziskemu
// executes them (no assembly emulator, no witness).
//
// | operation      | RISC-V                                              | a    | b    | rd   |
// |----------------|-----------------------------------------------------|------|------|------|
// | create (clean) | `csrs 0x822, addr` + `addi x0, size, CREATE_CLEAN`  | addr | size | -    |
// | create (dirty) | `csrs 0x822, addr` + `addi x0, size, CREATE_DIRTY`  | addr | size | -    |
// | alloc          | `csrrs rd, 0x822, size` + `addi x0, x0, ALLOC`      | size | 0    | addr |
// | calloc         | `csrrs rd, 0x822, size` + `addi x0, x0, CALLOC`     | size | 0    | addr |
// | dealloc        | `csrs 0x822, addr` + `addi x0, x0, DEALLOC`         | addr | 0    | -    |
//
// The address an alloc returns is a free value for the main, like the result of an fcall: the
// precompile is what proves it.

/// Allocator precompile: create an area of zeroed memory, `a` = address, `b` = size.
pub const ALLOC_OP_CREATE_CLEAN_ID: u8 = 16;
/// Allocator precompile: create an area of memory that is not zeroed, `a` = address, `b` = size.
pub const ALLOC_OP_CREATE_DIRTY_ID: u8 = 17;
/// Allocator precompile: allocate `a` bytes (a non-zero multiple of 8), the address goes to rd.
pub const ALLOC_OP_ALLOC_ID: u8 = 18;
/// Allocator precompile: allocate `a` zeroed bytes (a non-zero multiple of 8), the address goes
/// to rd.
pub const ALLOC_OP_CALLOC_ID: u8 = 19;
/// Allocator precompile: release the block that starts at `a`.
pub const ALLOC_OP_DEALLOC_ID: u8 = 20;

/// Whether a mark kind is an operation of the allocator precompile, not an analysis mark.
pub const fn is_alloc_op(kind: u8) -> bool {
    kind >= ALLOC_OP_CREATE_CLEAN_ID && kind <= ALLOC_OP_DEALLOC_ID
}

/// Whether an operation of the allocator precompile returns an address (in rd).
pub const fn alloc_op_has_result(kind: u8) -> bool {
    kind == ALLOC_OP_ALLOC_ID || kind == ALLOC_OP_CALLOC_ID
}
