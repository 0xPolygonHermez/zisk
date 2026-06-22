use super::kernel_heap::*;
use core::alloc::{GlobalAlloc, Layout};
use talc::{ErrOnOom, Talc, Talck};

static INNER_HEAP: Talck<talc::locking::AssumeUnlockable, ErrOnOom> = Talc::new(ErrOnOom).lock();

#[global_allocator]
static HEAP: TalcAlloc = TalcAlloc;

struct TalcAlloc;

unsafe impl GlobalAlloc for TalcAlloc {
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

pub fn init() {
    unsafe {
        let heap_start = &_kernel_heap_bottom as *const u8 as usize;
        let heap_size = &_kernel_heap_size as *const u8 as usize;
        let heap_span = talc::Span::from_base_size(heap_start as *mut u8, heap_size);
        INNER_HEAP.lock().claim(heap_span).unwrap();
        super::HEAP_POS = heap_start;
        super::HEAP_TOP = heap_start + heap_size;
    }
}
