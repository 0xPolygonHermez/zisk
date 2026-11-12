//! Kernel heap bounds.
//!
//! The heap starts where the linker script leaves the program image (`_heap_bottom`, the end of
//! `.bss`) and ends at the end of the RAM. Only the start comes from a linker symbol: the guest
//! is linked with the medium code model, so a symbol address must be within +-2GiB of the code,
//! which the end of the RAM (above the PC reach, see `core/src/mem.rs`) is not. The end is a
//! constant, materialised as an immediate.

use crate::ziskos_definitions::ziskos_config::{RAM_ADDR, RAM_SIZE};

extern "C" {
    pub static _heap_bottom: u8;
}

/// First address of the heap: the end of the program image.
#[inline(always)]
pub fn heap_bottom() -> usize {
    unsafe { &_heap_bottom as *const u8 as usize }
}

/// One past the last address of the heap: the end of the RAM.
#[inline(always)]
pub const fn heap_top() -> usize {
    (RAM_ADDR + RAM_SIZE) as usize
}

/// Size of the heap in bytes.
#[inline(always)]
pub fn heap_size() -> usize {
    heap_top() - heap_bottom()
}
