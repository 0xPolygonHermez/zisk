//! Kernel heap bounds.
//!
//! The heap starts where the linker script leaves the program image (`_heap_bottom`, the end of
//! `.bss`) and ends at `_heap_top`, the end of the RAM the ELF is linked for: 512 MB with
//! `zisk_linker_script.ld`, 2 GB with `zisk_linker_script_ram2gb.ld` (`cargo-zisk build
//! --ram-2gb`). It must not end beyond the RAM the prover reserves, or the first access there
//! faults: some allocators write at the top of their region when they start.
//!
//! The guest is linked with the medium code model, so the code can only take the address of a
//! symbol within +-2GiB of it. `_heap_bottom` is; `_heap_top` may not be (0x1_4000_0000 with 2 GB
//! of RAM, above the PC reach, see `core/src/mem.rs`), so the code reads it from a 64-bit word in
//! the read-only data, which the linker fills with an absolute relocation.
//!
//! Each allocator uses a subset of these bounds (the isolated staticlib's bump allocator, none),
//! so unused ones are not an error.
#![allow(dead_code)]

extern "C" {
    pub static _heap_bottom: u8;
    pub static _heap_top: u8;
}

/// First address of the heap: the end of the program image.
#[inline(always)]
pub fn heap_bottom() -> usize {
    unsafe { &_heap_bottom as *const u8 as usize }
}

/// One past the last address of the heap: the end of the RAM the ELF is linked for.
#[inline(always)]
pub fn heap_top() -> usize {
    /// `&_heap_top`, stored by the linker (absolute 64-bit relocation, any address).
    struct Addr(*const u8);
    // SAFETY: read-only and never dereferenced; only its address value is read.
    unsafe impl Sync for Addr {}
    static HEAP_TOP: Addr = Addr(unsafe { core::ptr::addr_of!(_heap_top) });
    // Volatile, so the compiler loads the word instead of folding it back into a PC-relative
    // address of `_heap_top`, which the link could not resolve.
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(HEAP_TOP.0)) as usize }
}

/// Size of the heap in bytes.
#[inline(always)]
pub fn heap_size() -> usize {
    heap_top() - heap_bottom()
}
