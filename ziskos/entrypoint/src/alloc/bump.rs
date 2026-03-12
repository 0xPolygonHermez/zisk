use core::alloc::{GlobalAlloc, Layout};

use crate::alloc::{sys_alloc_aligned, sys_alloc_log};
use crate::ziskos_memcpy;

#[global_allocator]
pub static HEAP: BumpPointerAlloc = BumpPointerAlloc;

pub struct BumpPointerAlloc;

unsafe impl GlobalAlloc for BumpPointerAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        sys_alloc_aligned(layout.size(), layout.align())
        // let ptr = sys_alloc_aligned(layout.size(), layout.align());
        // sys_alloc_log(0, ptr, layout.size(), layout.align());
        // ptr
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // sys_alloc_log(1, ptr, layout.size(), layout.align())
        // this allocator never deallocates memory
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.alloc(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());

        let new_ptr = self.alloc(new_layout);

        if !new_ptr.is_null() {
            let copy_size = layout.size().min(new_size);
            ziskos_memcpy!(ptr: new_ptr, ptr, copy_size);
        }

        new_ptr
    }
}
