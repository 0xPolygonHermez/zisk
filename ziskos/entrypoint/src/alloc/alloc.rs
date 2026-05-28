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

    debug_assert!(align.is_power_of_two(), "align must be a power of two");

    // `align - 1` is safe because align >= 1 (enforced by debug_assert above).
    let offset = heap_pos & (align - 1);
    if offset != 0 {
        heap_pos = heap_pos.checked_add(align - offset).expect("heap_pos alignment overflow");
    }

    let ptr = heap_pos as *mut u8;

    // Guard against integer overflow in the size addition *before* the OOM check.
    // Without this, a large `bytes` value wraps heap_pos to a tiny number, the
    // OOM check passes on the wrapped value, and HEAP_POS is corrupted.
    heap_pos = heap_pos.checked_add(bytes).expect("allocation size overflow");

    // Check to make sure heap doesn't collide with SYSTEM memory.
    if unsafe { HEAP_TOP } < heap_pos {
        panic!("OOM limit of heap with bump allocator");
    }

    unsafe { HEAP_POS = heap_pos };

    ptr
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::ptr;

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
