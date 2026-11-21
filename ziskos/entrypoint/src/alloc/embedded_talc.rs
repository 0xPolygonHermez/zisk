use super::kernel_heap::*;
use core::alloc::{GlobalAlloc, Layout};
use talc::{ErrOnOom, Talc, Talck};

#[cfg_attr(not(feature = "alloc-marks"), global_allocator)]
static HEAP: Talck<talc::locking::AssumeUnlockable, ErrOnOom> = Talc::new(ErrOnOom).lock();

/// With `alloc-marks` the global allocator is a marking front end of `HEAP`.
#[cfg(feature = "alloc-marks")]
struct MarkedHeap;

#[cfg(feature = "alloc-marks")]
#[global_allocator]
static MARKED_HEAP: super::AllocMarks<MarkedHeap> = super::AllocMarks(MarkedHeap);

#[cfg(feature = "alloc-marks")]
unsafe impl GlobalAlloc for MarkedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        HEAP.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HEAP.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        HEAP.realloc(ptr, layout, new_size)
    }
}

pub fn init() {
    unsafe {
        let heap_start = &_heap_bottom as *const u8 as usize;
        let heap_size = &_heap_size as *const u8 as usize;
        let heap_span = talc::Span::from_base_size(heap_start as *mut u8, heap_size);
        HEAP.lock().claim(heap_span).unwrap();
    }
}
