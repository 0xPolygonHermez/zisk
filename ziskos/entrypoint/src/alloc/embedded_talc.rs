use super::kernel_heap::*;
use core::alloc::{GlobalAlloc, Layout};
use talc::{ErrOnOom, Talc, Talck};

#[global_allocator]
static HEAP: Talck<talc::locking::AssumeUnlockable, ErrOnOom> = Talc::new(ErrOnOom).lock();

pub fn init() {
    unsafe {
        let heap_start = &_kernel_heap_bottom as *const u8 as usize;
        let heap_size = &_kernel_heap_size as *const u8 as usize;
        let heap_span = talc::Span::from_base_size(heap_start as *mut u8, heap_size);
        HEAP.lock().claim(heap_span).unwrap();
    }
}
