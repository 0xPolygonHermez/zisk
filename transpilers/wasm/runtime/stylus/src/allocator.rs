//! The interpreter survives imports; its heap cannot share Brotli's resettable arena.
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};
use dlmalloc::{Allocator, Dlmalloc};

unsafe extern "C" {
    static __stylus_heap_start: u8;
    static __stylus_heap_end: u8;
    fn runtime_trap() -> !;
}

struct Heap;
static mut NEXT: usize = 0;
static mut END: usize = 0;

// All helper entry points execute on the same thread.
unsafe impl Allocator for Heap {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        unsafe {
            let start = NEXT;
            let Some(end) = start.checked_add(size) else { return (ptr::null_mut(), 0, 0) };
            if end > END {
                return (ptr::null_mut(), 0, 0);
            }
            NEXT = end;
            (start as *mut u8, size, 0)
        }
    }
    fn remap(&self, _: *mut u8, _: usize, _: usize, _: bool) -> *mut u8 {
        ptr::null_mut()
    }
    fn free_part(&self, _: *mut u8, _: usize, _: usize) -> bool {
        false
    }
    fn free(&self, _: *mut u8, _: usize) -> bool {
        false
    }
    fn can_release_part(&self, _: u32) -> bool {
        false
    }
    fn allocates_zeros(&self) -> bool {
        false
    }
    fn page_size(&self) -> usize {
        4096
    }
}

static mut MALLOC: Dlmalloc<Heap> = Dlmalloc::new_with_allocator(Heap);
struct Global;
#[global_allocator]
static GLOBAL: Global = Global;

unsafe impl GlobalAlloc for Global {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { (&mut *ptr::addr_of_mut!(MALLOC)).malloc(layout.size(), layout.align()) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { (&mut *ptr::addr_of_mut!(MALLOC)).free(ptr, layout.size(), layout.align()) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { (&mut *ptr::addr_of_mut!(MALLOC)).calloc(layout.size(), layout.align()) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        unsafe {
            (&mut *ptr::addr_of_mut!(MALLOC)).realloc(ptr, layout.size(), layout.align(), size)
        }
    }
}

pub fn initialize() {
    unsafe {
        if END != 0 {
            return;
        }
        let start = ptr::addr_of!(__stylus_heap_start) as usize;
        let end = ptr::addr_of!(__stylus_heap_end) as usize;
        if start == 0 || start >= end || start % 4096 != 0 || end % 4096 != 0 {
            runtime_trap();
        }
        NEXT = start;
        END = end;
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sys_alloc_aligned(size: usize, align: usize) -> *mut u8 {
    let Ok(layout) = Layout::from_size_align(size, align) else { unsafe { runtime_trap() } };
    let result = unsafe { GLOBAL.alloc(layout) };
    if result.is_null() {
        unsafe { runtime_trap() }
    }
    result
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sys_write(_: u32, data: *const u8, len: usize) {
    for i in 0..len {
        unsafe {
            ptr::write_volatile(0xa0400200 as *mut u8, ptr::read(data.add(i)));
        }
    }
}

macro_rules! unavailable {
    ($($name:ident),+ $(,)?) => { $(
        #[unsafe(no_mangle)]
        extern "C" fn $name() -> ! { unsafe { runtime_trap() } }
    )+ };
}
unavailable!(sys_alloc_words, sys_argc, sys_argv, sys_read);

#[unsafe(no_mangle)]
extern "C" fn sys_getenv(_: *mut u32, _: usize, _: *const u8, _: usize) -> usize {
    usize::MAX
}

// Only the native interpreter's standard-library hash-table seeding uses this hook.
#[unsafe(no_mangle)]
unsafe extern "C" fn sys_rand(data: *mut u8, len: usize) {
    static mut SEED: u64 = 0x123456789abcdef0;
    for i in 0..len {
        unsafe {
            SEED ^= SEED << 13;
            SEED ^= SEED >> 7;
            SEED ^= SEED << 17;
            ptr::write(data.add(i), SEED as u8);
        }
    }
}
