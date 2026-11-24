//! Allocation marks (`alloc-marks` feature).
//!
//! [`AllocMarks`] wraps a global allocator and tells the emulator what every call does through
//! allocation marks: no-op instructions that carry the sizes, alignments and pointers of the
//! call (see `zisk_definitions::alloc_mark`). `ziskemu --alloc-stats` turns them into an analysis
//! of how the program uses its allocator.
//!
//! Every mark is one extra step (a `copyb` that stores nothing, which the prover sees as a no-op),
//! so a guest built with them can still be proven but is slightly longer: the feature is meant
//! for analysis builds.
//!
//! The allocators of ziskos are wrapped when the feature is on. A guest that brings its own
//! (`zisk-custom-alloc`) can wrap it the same way:
//!
//! ```ignore
//! #[global_allocator]
//! static HEAP: ziskos::AllocMarks<MyAlloc> = ziskos::AllocMarks(MyAlloc);
//! ```

use core::alloc::{GlobalAlloc, Layout};

use zisk_definitions::{
    ALLOC_MARK_ALLOC_ID, ALLOC_MARK_ALLOC_ZEROED_ID, ALLOC_MARK_DEALLOC_ID, ALLOC_MARK_END_ID,
    ALLOC_MARK_LAYOUT_ID, ALLOC_MARK_REALLOC_ID, SYSCALL_ALLOC_MARK_ID,
};

use crate::ziskos_syscall;

/// Emits an allocation mark of kind `$kind` with one or two values.
macro_rules! alloc_mark {
    ($kind:expr, $a:expr) => {
        ziskos_syscall!(SYSCALL_ALLOC_MARK_ID, $kind, $a as usize)
    };
    ($kind:expr, $a:expr, $b:expr) => {
        ziskos_syscall!(SYSCALL_ALLOC_MARK_ID, $kind, $a as usize, $b as usize)
    };
}

/// A global allocator that marks every call for `ziskemu --alloc-stats`.
pub struct AllocMarks<A>(pub A);

unsafe impl<A: GlobalAlloc> GlobalAlloc for AllocMarks<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        alloc_mark!(ALLOC_MARK_ALLOC_ID, layout.size(), layout.align());
        let ptr = self.0.alloc(layout);
        alloc_mark!(ALLOC_MARK_END_ID, ptr);
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        alloc_mark!(ALLOC_MARK_ALLOC_ZEROED_ID, layout.size(), layout.align());
        let ptr = self.0.alloc_zeroed(layout);
        alloc_mark!(ALLOC_MARK_END_ID, ptr);
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        alloc_mark!(ALLOC_MARK_LAYOUT_ID, layout.size(), layout.align());
        alloc_mark!(ALLOC_MARK_DEALLOC_ID, ptr);
        self.0.dealloc(ptr, layout);
        alloc_mark!(ALLOC_MARK_END_ID, 0usize);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        alloc_mark!(ALLOC_MARK_LAYOUT_ID, layout.size(), layout.align());
        alloc_mark!(ALLOC_MARK_REALLOC_ID, ptr, new_size);
        let new_ptr = self.0.realloc(ptr, layout, new_size);
        alloc_mark!(ALLOC_MARK_END_ID, new_ptr);
        new_ptr
    }
}
