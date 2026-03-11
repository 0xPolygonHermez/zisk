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

pub fn init() {
    unsafe {
        let heap_start = &_kernel_heap_bottom as *const u8 as usize;
        let heap_size = &_kernel_heap_size as *const u8 as usize;
        HEAP.init(heap_start, heap_size)
    }
}
