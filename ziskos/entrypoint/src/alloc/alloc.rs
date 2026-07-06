#[used]
#[export_name = "ZISK_BUMP_HEAP_POS"]
static mut HEAP_POS: usize = 0;

#[used]
#[export_name = "ZISK_BUMP_HEAP_TOP"]
static mut HEAP_TOP: usize = 0;

#[cfg(zisk_guest)]
#[no_mangle]
#[warn(dead_code)]
pub unsafe extern "C" fn init_sys_alloc() {
    let (bottom, top) = sys_alloc_heap_bounds();
    unsafe {
        HEAP_POS = bottom;
        HEAP_TOP = top;
    };
}

#[cfg(all(zisk_guest, zisk_staticlib))]
#[no_mangle]
#[warn(dead_code)]
pub unsafe extern "C" fn reset_sys_alloc() {
    let (bottom, top) = sys_alloc_heap_bounds();
    unsafe {
        HEAP_POS = bottom;
        HEAP_TOP = top;
        #[cfg(feature = "alloc-stats")]
        {
            HEAP_BOTTOM = bottom;
        }
    };
}

// ---------------------------------------------------------------------------
// Isolated-allocator usage statistics (`alloc-stats` feature).
//
// The bump allocator only grows within a single host-facing call (it never
// frees) and `reset_sys_alloc` rewinds it to `HEAP_BOTTOM` at the start of the
// next call. So the peak usage of each call is `HEAP_POS - HEAP_BOTTOM` measured
// right before returning; `update_max_used_sys_alloc` folds that into the
// program-wide maximum. Single-threaded, so plain `static mut` is sufficient.
// ---------------------------------------------------------------------------

/// Heap base recorded on the last `reset_sys_alloc`, used to compute usage.
#[cfg(all(zisk_guest, zisk_staticlib, feature = "alloc-stats"))]
static mut HEAP_BOTTOM: usize = 0;

/// Program-wide peak bytes handed out by the isolated bump allocator.
#[cfg(all(zisk_guest, zisk_staticlib, feature = "alloc-stats"))]
static mut HEAP_MAX_USED: usize = 0;

/// Fold the current heap usage into the running maximum. Called by the
/// `wrap_export!` wrappers after each host-facing call returns.
#[cfg(all(zisk_guest, zisk_staticlib, feature = "alloc-stats"))]
#[no_mangle]
pub unsafe extern "C" fn update_max_used_sys_alloc() {
    unsafe {
        let used = HEAP_POS.saturating_sub(HEAP_BOTTOM);
        if used > HEAP_MAX_USED {
            HEAP_MAX_USED = used;
        }
    }
}

/// Query the program-wide peak bytes used by the isolated bump allocator.
#[cfg(all(zisk_guest, zisk_staticlib, feature = "alloc-stats"))]
#[no_mangle]
pub unsafe extern "C" fn get_max_used_sys_alloc() -> usize {
    unsafe { HEAP_MAX_USED }
}

/// Print the peak usage to the UART, e.g. `ziskos-isolated-allocator use: 1234 bytes`.
#[cfg(all(zisk_guest, zisk_staticlib, feature = "alloc-stats"))]
pub unsafe fn print_max_used_sys_alloc() {
    use crate::ziskos::{sys_write, sys_write_u64};
    let prefix = b"ziskos-isolated-allocator use: ";
    let suffix = b" bytes\n";
    unsafe {
        sys_write(1, prefix.as_ptr(), prefix.len());
        sys_write_u64(get_max_used_sys_alloc() as u64, false);
        sys_write(1, suffix.as_ptr(), suffix.len());
    }
}

/// Backing region for the bump allocator, as `(bottom, top)` addresses.
///
/// Guest binary (default): the heap is the RAM left over after the program
/// image, delimited by the linker symbols `_heap_*`.
#[cfg(all(zisk_guest, not(zisk_staticlib)))]
unsafe fn sys_alloc_heap_bounds() -> (usize, usize) {
    extern "C" {
        static _heap_bottom: u8;
        static _heap_top: u8;
    }

    unsafe { (&_heap_bottom as *const u8 as usize, &_heap_top as *const u8 as usize) }
}

/// Isolated staticlib (`zisk_staticlib`): ziskos is linked into a host
/// application that owns the linker script, so `_heap_*` may not exist.
/// Carve the heap out of a static buffer compiled into `libziskos.a`, keeping
/// ziskos's heap fully isolated from the host's memory.
///
/// The buffer lives in `.bss` (zero-initialized: it does not bloat the archive
/// on disk, but the host reserves this much address space at load time). Adjust
/// `HEAP_SIZE` to the largest working set ziskos needs inside the host.
#[cfg(all(zisk_guest, zisk_staticlib))]
unsafe fn sys_alloc_heap_bounds() -> (usize, usize) {
    // 8 MiB chosen empirically: measuring 60 MiB "killer blocks" with the
    // `alloc-stats` feature, the peak usage of this bump allocator was just over
    // 1 MiB. 8 MiB leaves a comfortable safety margin over that observed peak
    // while keeping the reserved `.bss` address space small.
    const HEAP_SIZE: usize = 8 * 1024 * 1024;

    // Only ever accessed by address (the bump allocator re-aligns each block),
    // so the inner array is never "read" in the borrow-checker's sense.
    #[allow(dead_code)]
    #[repr(align(8))]
    struct Heap([u8; HEAP_SIZE]);

    static mut HEAP: Heap = Heap([0; HEAP_SIZE]);

    let start = core::ptr::addr_of_mut!(HEAP) as usize;
    (start, start + HEAP_SIZE)
}

#[cfg(zisk_guest)]
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn sys_alloc_aligned(bytes: usize, align: usize) -> *mut u8 {
    inline_bump_alloc_aligned(bytes, align)
}

#[cfg(zisk_guest)]
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

#[cfg(zisk_guest)]
use core::ptr;

#[cfg(zisk_guest)]
#[no_mangle]
static mut SINK: u64 = 0;

#[cfg(zisk_guest)]
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn sys_alloc_log(op: u64, ptr: *mut u8, bytes: usize, align: usize) {
    unsafe {
        ptr::write_volatile(&raw mut SINK, bytes as u64 + op + (ptr as u64 & 0x02) + align as u64);
    }
}
