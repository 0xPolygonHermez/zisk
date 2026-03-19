#[used]
#[export_name = "ZISK_BUMP_HEAP_POS"]
static mut HEAP_POS: usize = 0;

#[used]
#[export_name = "ZISK_BUMP_HEAP_TOP"]
static mut HEAP_TOP: usize = 0;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[no_mangle]
#[warn(dead_code)]
pub unsafe extern "C" fn init_sys_alloc() {
    extern "C" {
        static _kernel_heap_bottom: u8;
        static _kernel_heap_top: u8;
    }

    unsafe {
        HEAP_POS = &_kernel_heap_bottom as *const u8 as usize;
        HEAP_TOP = &_kernel_heap_top as *const u8 as usize;
    };
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn sys_alloc_aligned(bytes: usize, align: usize) -> *mut u8 {
    inline_bump_alloc_aligned(bytes, align)
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub unsafe fn inline_bump_alloc_aligned(bytes: usize, align: usize) -> *mut u8 {
    // SAFETY: Single threaded, so nothing else can touch this while we're working.
    let mut heap_pos = unsafe { HEAP_POS };

    let offset = heap_pos & (align - 1);
    if offset != 0 {
        heap_pos += align - offset;
    }

    let ptr = heap_pos as *mut u8;
    heap_pos += bytes;

    // Check to make sure heap doesn't collide with SYSTEM memory.
    if HEAP_TOP < heap_pos {
        panic!("OOM limit of heap with bump allocator");
    }

    unsafe { HEAP_POS = heap_pos };

    ptr
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use std::ptr;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[no_mangle]
static mut SINK: u64 = 0;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn sys_alloc_log(op: u64, ptr: *mut u8, bytes: usize, align: usize) {
    unsafe {
        ptr::write_volatile(&raw mut SINK, bytes as u64 + op + (ptr as u64 & 0x02) + align as u64);
    }
}
