use crate::ziskos::sys_write;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::addr_of_mut;
use dlmalloc::{Allocator as DlAllocator, Dlmalloc};

use super::kernel_heap::*;

// Implementar el backend que le da memoria a dlmalloc
struct ZiskSystem;

unsafe impl DlAllocator for ZiskSystem {
    // Equivalente a sbrk — dlmalloc pide más memoria aquí
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        unsafe {
            // Devuelves un bloque de tu heap reservado
            let ptr = BUMP_PTR;
            let aligned = (ptr + 7) & !7;
            BUMP_PTR = aligned + size;
            if BUMP_PTR > BUMP_END {
                return (core::ptr::null_mut(), 0, 0);
            }
            (aligned as *mut u8, size, 0)
        }
    }

    fn remap(&self, _ptr: *mut u8, _oldsize: usize, _newsize: usize, _can_move: bool) -> *mut u8 {
        core::ptr::null_mut() // no soportado
    }

    fn free_part(&self, _ptr: *mut u8, _oldsize: usize, _newsize: usize) -> bool {
        false // no soportado
    }

    fn free(&self, _ptr: *mut u8, _size: usize) -> bool {
        false // devolver memoria al sistema — no necesario
    }

    fn can_release_part(&self, _flags: u32) -> bool {
        false
    }

    fn allocates_zeros(&self) -> bool {
        false
    }

    fn page_size(&self) -> usize {
        4096
    }
}

static mut BUMP_PTR: usize = 0;
static mut BUMP_END: usize = 0;

static mut DLMALLOC: Dlmalloc<ZiskSystem> = Dlmalloc::new_with_allocator(ZiskSystem);

struct Allocator;

#[global_allocator]
static GLOBAL: Allocator = Allocator;

unsafe impl core::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        (*addr_of_mut!(DLMALLOC)).malloc(layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        (*addr_of_mut!(DLMALLOC)).free(ptr, layout.size(), layout.align())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        (*addr_of_mut!(DLMALLOC)).calloc(layout.size(), layout.align())
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        (*addr_of_mut!(DLMALLOC)).realloc(ptr, layout.size(), layout.align(), new_size)
    }
}

pub fn init() {
    unsafe {
        let heap_start = &_kernel_heap_bottom as *const u8 as usize;
        let heap_end = &_kernel_heap_top as *const u8 as usize;
        BUMP_PTR = heap_start;
        BUMP_END = heap_end;
    }
}
