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
