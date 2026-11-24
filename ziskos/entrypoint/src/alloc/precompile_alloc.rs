//! Global allocator over the allocator precompile (`zisk-precompile-alloc` feature, PROTOTYPE).
//!
//! The heap is handed to the precompile once, as one clean area, and every allocation, zeroed
//! allocation and release is one operation of the precompile (see
//! `zisk_definitions::alloc_mark`). Only ziskemu executes these operations (`--alloc-policy`
//! chooses the variant): a guest built with this feature cannot run on the assembly emulator nor
//! be proven yet.
//!
//! The precompile serves blocks of a non-zero multiple of 8 bytes, aligned to 8:
//!
//! - Sizes are rounded up to 8 bytes, a zero size counts as 8.
//! - An alignment bigger than 8 asks for `align` extra bytes, returns the first aligned address
//!   with room for a word in front of it, and keeps there the start of the block, which the
//!   release (it knows the layout) reads back.
//! - `realloc` is `alloc` + `memcpy` + `dealloc`, unless the rounded size does not change.

use core::alloc::{GlobalAlloc, Layout};

use zisk_definitions::{
    ALLOC_OP_ALLOC_ID, ALLOC_OP_CALLOC_ID, ALLOC_OP_CREATE_CLEAN_ID, ALLOC_OP_DEALLOC_ID,
    SYSCALL_ALLOC_MARK_ID,
};

use super::kernel_heap::*;
use crate::ziskos_memcpy;

/// Gives `[addr, addr + size)` to the precompile as an area of zeroed memory.
#[inline(always)]
unsafe fn create_clean(addr: usize, size: usize) {
    core::arch::asm!(
        "csrs {port}, {addr}",
        "addi x0, {size}, {kind}",
        port = const SYSCALL_ALLOC_MARK_ID,
        addr = in(reg) addr,
        size = in(reg) size,
        kind = const ALLOC_OP_CREATE_CLEAN_ID,
        options(nostack)
    );
}

/// Allocates `size` bytes (a non-zero multiple of 8); zeroed if `KIND` is `ALLOC_OP_CALLOC_ID`.
#[inline(always)]
unsafe fn alloc_op<const KIND: u8>(size: usize) -> *mut u8 {
    let addr: usize;
    core::arch::asm!(
        "csrrs {addr}, {port}, {size}",
        "addi x0, x0, {kind}",
        port = const SYSCALL_ALLOC_MARK_ID,
        addr = out(reg) addr,
        size = in(reg) size,
        kind = const KIND,
        options(nostack)
    );
    addr as *mut u8
}

/// Releases the block that starts at `addr`.
#[inline(always)]
unsafe fn dealloc_op(addr: *mut u8) {
    core::arch::asm!(
        "csrs {port}, {addr}",
        "addi x0, x0, {kind}",
        port = const SYSCALL_ALLOC_MARK_ID,
        addr = in(reg) addr,
        kind = const ALLOC_OP_DEALLOC_ID,
        options(nostack)
    );
}

/// Size asked to the precompile for `size` bytes.
#[inline(always)]
fn block_size(size: usize) -> usize {
    (size.max(1) + 7) & !7
}

pub struct PrecompileAlloc;

impl PrecompileAlloc {
    #[inline(always)]
    unsafe fn allocate<const KIND: u8>(layout: Layout) -> *mut u8 {
        let align = layout.align();
        if align <= 8 {
            return alloc_op::<KIND>(block_size(layout.size()));
        }
        // Room to align plus a word in front of the aligned address for the start of the block.
        let base = alloc_op::<KIND>(block_size(layout.size()) + align) as usize;
        let ptr = (base + 8 + align - 1) & !(align - 1);
        *((ptr - 8) as *mut usize) = base;
        ptr as *mut u8
    }
}

unsafe impl GlobalAlloc for PrecompileAlloc {
    #[inline(always)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::allocate::<ALLOC_OP_ALLOC_ID>(layout)
    }

    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // the precompile clears the block when it has been used before
        Self::allocate::<ALLOC_OP_CALLOC_ID>(layout)
    }

    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.align() <= 8 {
            dealloc_op(ptr);
        } else {
            dealloc_op(*((ptr as usize - 8) as *const usize) as *mut u8);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if block_size(new_size) == block_size(layout.size()) {
            return ptr;
        }
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);
        let copy_size = layout.size().min(new_size);
        ziskos_memcpy!(ptr: new_ptr, ptr, copy_size);
        self.dealloc(ptr, layout);
        new_ptr
    }
}

#[cfg(not(feature = "alloc-marks"))]
#[global_allocator]
static HEAP: PrecompileAlloc = PrecompileAlloc;

#[cfg(feature = "alloc-marks")]
#[global_allocator]
static HEAP: super::AllocMarks<PrecompileAlloc> = super::AllocMarks(PrecompileAlloc);

/// Hands the heap to the precompile. Called once, before `main`.
pub fn init() {
    unsafe {
        let bottom = (&_heap_bottom as *const u8 as usize + 7) & !7;
        let top = &_heap_top as *const u8 as usize & !7;
        create_clean(bottom, top - bottom);
    }
}
