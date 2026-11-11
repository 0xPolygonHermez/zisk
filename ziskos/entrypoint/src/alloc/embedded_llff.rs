use core::alloc::{GlobalAlloc, Layout};
use critical_section::RawRestoreState;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

struct CriticalSection;
critical_section::set_impl!(CriticalSection);

unsafe impl critical_section::Impl for CriticalSection {
    unsafe fn acquire() -> RawRestoreState {}
    unsafe fn release(_token: RawRestoreState) {}
}

use super::kernel_heap::*;

pub fn init() {
    unsafe {
        let heap_start = heap_bottom();
        let heap_size = heap_size();
        HEAP.init(heap_start, heap_size)
    }
}
