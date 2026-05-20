use core::alloc::{GlobalAlloc, Layout};
use core::ptr::addr_of_mut;
use linked_list_allocator::Heap;

use super::kernel_heap::*;

static mut HEAP: Heap = Heap::empty();

struct Allocator;

#[global_allocator]
static GLOBAL: Allocator = Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        (*addr_of_mut!(HEAP))
            .allocate_first_fit(layout)
            .map(|p| p.as_ptr())
            .unwrap_or(core::ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(nn) = core::ptr::NonNull::new(ptr) {
            (*addr_of_mut!(HEAP)).deallocate(nn, layout);
        }
    }
}

pub fn init() {
    unsafe {
        let heap_start = &_kernel_heap_bottom as *const u8 as usize;
        let heap_size = &_kernel_heap_size as *const u8 as usize;
        (*addr_of_mut!(HEAP)).init(heap_start as *mut u8, heap_size);
    }
}
