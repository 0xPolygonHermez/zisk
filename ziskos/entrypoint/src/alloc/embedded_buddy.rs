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
        let heap_start = heap_bottom();
        let heap_size = heap_size();
        INNER_HEAP.init(heap_start, heap_size);
    }
}

struct EmbeddedAlloc;

unsafe impl GlobalAlloc for EmbeddedAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        INNER_HEAP.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        INNER_HEAP.dealloc(ptr, layout)
    }
}
