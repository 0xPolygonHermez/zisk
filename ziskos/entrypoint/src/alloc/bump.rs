use core::alloc::{GlobalAlloc, Layout};

use crate::alloc::{inline_bump_alloc_aligned, sys_alloc_aligned, sys_alloc_log};
use crate::ziskos_memcpy;

#[global_allocator]
pub static HEAP: BumpPointerAlloc = BumpPointerAlloc;

pub struct BumpPointerAlloc;

unsafe impl GlobalAlloc for BumpPointerAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        inline_bump_alloc_aligned(layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // sys_alloc_log(1, ptr, layout.size(), layout.align())
        // this allocator never deallocates memory
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        inline_bump_alloc_aligned(layout.size(), layout.align())
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = inline_bump_alloc_aligned(new_size, layout.align());
        // OOM => panic on allocation
        let copy_size = layout.size().min(new_size);
        ziskos_memcpy!(ptr: new_ptr, ptr, copy_size);

        new_ptr
    }
}
