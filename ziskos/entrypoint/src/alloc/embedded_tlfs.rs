use core::alloc::{GlobalAlloc, Layout};
use critical_section::RawRestoreState;
use embedded_alloc::TlsfHeap as Heap;

use super::kernel_heap::*;

#[global_allocator]
static HEAP: EmbeddedAlloc = EmbeddedAlloc;

static INNER_HEAP: Heap = Heap::empty();

struct CriticalSection;
critical_section::set_impl!(CriticalSection);

unsafe impl critical_section::Impl for CriticalSection {
    unsafe fn acquire() -> RawRestoreState {}
    unsafe fn release(_token: RawRestoreState) {}
}

pub fn init() {
    unsafe {
        let heap_start = &_kernel_heap_bottom as *const u8 as usize;
        let heap_size = &_kernel_heap_size as *const u8 as usize;
        INNER_HEAP.init(heap_start, heap_size);
        super::HEAP_POS = heap_start;
        super::HEAP_TOP = heap_start + heap_size;
    }
}

struct EmbeddedAlloc;

unsafe impl GlobalAlloc for EmbeddedAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = INNER_HEAP.alloc(layout);
        if !ptr.is_null() {
            let end = ptr as usize + layout.size();
            if end > super::HEAP_POS {
                super::HEAP_POS = end;
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        INNER_HEAP.dealloc(ptr, layout)
    }
}
