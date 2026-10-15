/// Builds an EVM `JUMPDEST` bitmap from bytecode using a custom CSR syscall.
///
/// This macro mirrors the `ziskos_memcpy!` calling style: destination + source + size.
///
/// # Arguments
/// * `$bitmap` - Mutable bitmap destination (slice/array of `u64` words)
/// * `$bytecode` - Source bytecode (slice/array of `u8`)
/// * `$size` - Number of bytecode bytes to process
///
/// # Preconditions
///
/// The precompile can only be proven for calls that meet all three. The first two
/// are the caller's to check; the third this macro handles.
///
/// * `bytecode` is 8-byte aligned. The machine reads whole aligned 64-bit words; an
///   unaligned run is not arithmetizable. Check it and use a software walk
///   otherwise. `bitmap` may be unaligned: the macro then builds the bitmap in an
///   aligned buffer and copies it out.
/// * `bitmap` holds at least `size.div_ceil(64)` words. The last word is written in
///   full even when the bytecode ends part way into it.
/// * `size > 0`. An empty call spans no bitmap word, so it occupies no row in the
///   AIR and nothing would prove it while main still claims it — the operation bus
///   would not balance. The macro skips the syscall instead, which is also the right
///   answer: there is nothing to compute and `bytecode` may not even be a live
///   pointer.
///
/// Breaking one of these is not a soundness matter. It leaves the program unprovable,
/// and the emulator asserts on it so the bug shows up there rather than at proving
/// time.
#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_jump_dest {
    ($bitmap:expr, $bytecode:expr, $size:expr) => {
        $crate::ziskos_jump_dest!(ptr: $bitmap.as_mut_ptr(), $bytecode.as_ptr(), $size)
    };
    (dst_ptr: $bitmap:expr, $bytecode:expr, $size:expr) => {
        $crate::ziskos_jump_dest!(ptr: $bitmap, $bytecode.as_ptr(), $size)
    };
    (ptr: $bitmap:expr, $bytecode:expr, $size:expr) => {{
        let size: usize = $size;
        if size != 0 {
            let dst = $bitmap;
            let src = $bytecode;
            if (dst as usize) & 7 == 0 {
                unsafe {
                    core::arch::asm!(
                        "csrs {port}, {src}",
                        "add x0, {dst}, {size}",
                        port = const zisk_definitions::SYSCALL_JUMP_DEST_ID,
                        size = in(reg) size,
                        dst = in(reg) dst,
                        src = in(reg) src,
                        options(nostack, preserves_flags),
                    );
                }
            } else {
                // The precompile writes the bitmap in whole aligned words: build it
                // in an aligned buffer and copy it to the caller's.
                unsafe { $crate::jump_dest_bitmap_unaligned(dst as *mut u8, src as *const u8, size) };
            }
        }
    }};
}

/// The JUMPDEST bitmap of `size` (> 0) bytes at `bytecode`, written to a `bitmap`
/// that is not 8-byte aligned: the precompile can only be proven writing aligned
/// words, so it runs on an aligned heap buffer of `size.div_ceil(64)` words, which
/// is then copied to `bitmap`. Called by [`ziskos_jump_dest!`]; not for direct use.
///
/// # Safety
/// `bitmap` must be valid for writing `size.div_ceil(64) * 8` bytes, and `bytecode`
/// for reading `size` bytes and 8-byte aligned.
#[cfg(zisk_guest)]
#[doc(hidden)]
#[inline(never)]
pub unsafe fn jump_dest_bitmap_unaligned(bitmap: *mut u8, bytecode: *const u8, size: usize) {
    let words = size.div_ceil(64);
    let mut tmp = crate::alloc_extern::vec![0u64; words];
    let tmp_ptr = tmp.as_mut_ptr();
    core::arch::asm!(
        "csrs {port}, {src}",
        "add x0, {dst}, {size}",
        port = const zisk_definitions::SYSCALL_JUMP_DEST_ID,
        size = in(reg) size,
        dst = in(reg) tmp_ptr,
        src = in(reg) bytecode,
        options(nostack, preserves_flags),
    );
    core::ptr::copy_nonoverlapping(tmp_ptr as *const u8, bitmap, words * 8);
}
