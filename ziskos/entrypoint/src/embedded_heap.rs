use alloc::alloc::{GlobalAlloc, Layout};
use critical_section::RawRestoreState;
use embedded_alloc::TlsfHeap as Heap;

#[global_allocator]
static HEAP: EmbeddedAlloc = EmbeddedAlloc;

static INNER_HEAP: Heap = Heap::empty();

struct CriticalSection;
critical_section::set_impl!(CriticalSection);

unsafe impl critical_section::Impl for CriticalSection {
    unsafe fn acquire() -> RawRestoreState {}
    unsafe fn release(_token: RawRestoreState) {}
}

extern "C" {
    static _kernel_heap_bottom: u8;
    static _kernel_heap_size: u8;
}

pub fn init() {
    unsafe {
        let heap_start = &_kernel_heap_bottom as *const u8 as usize;
        //FIXME: this is a hack to get the size of the heap until the linker script is fixed
        let heap_size = 0x7FE0000_usize - (heap_start - 0xa0020000);
        INNER_HEAP.init(heap_start, heap_size);
    }

    println!("Embedded heap initialized (no dealloc)");
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
