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
/// The precompile can only be proven for calls that meet all three. The second is
/// the caller's to check; the macro handles the other two.
///
/// * `bitmap` and `bytecode` are 8-byte aligned. The machine reads and writes whole
///   aligned 64-bit words; an unaligned run is not arithmetizable. The macro checks
///   it, and otherwise runs the precompile on aligned copies (a heap allocation and
///   a copy of each unaligned operand).
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
            if ((dst as usize) | (src as usize)) & 7 == 0 {
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
                // The precompile reads the bytecode and writes the bitmap in whole
                // aligned words: run it on aligned copies.
                unsafe { $crate::jump_dest_bitmap_unaligned(dst as *mut u8, src as *const u8, size) };
            }
        }
    }};
}

/// The JUMPDEST bitmap of `size` (> 0) bytes at `bytecode`, written to `bitmap`,
/// when either is not 8-byte aligned: the precompile can only be proven reading and
/// writing aligned words, so an unaligned bytecode is first copied to an aligned
/// heap buffer, and an unaligned bitmap is built in one of `size.div_ceil(64)` words
/// and then copied out. Called by [`ziskos_jump_dest!`]; not for direct use.
///
/// # Safety
/// `bitmap` must be valid for writing `size.div_ceil(64) * 8` bytes, and `bytecode`
/// for reading `size` bytes.
#[cfg(zisk_guest)]
#[doc(hidden)]
#[inline(never)]
pub unsafe fn jump_dest_bitmap_unaligned(bitmap: *mut u8, bytecode: *const u8, size: usize) {
    let code_copy;
    let bytecode = if (bytecode as usize) & 7 == 0 {
        bytecode
    } else {
        let mut copy = crate::alloc_extern::vec![0u64; size.div_ceil(8)];
        core::ptr::copy_nonoverlapping(bytecode, copy.as_mut_ptr() as *mut u8, size);
        code_copy = copy;
        code_copy.as_ptr() as *const u8
    };
    let words = size.div_ceil(64);
    let mut out_copy;
    let tmp_ptr = if (bitmap as usize) & 7 == 0 {
        bitmap as *mut u64
    } else {
        out_copy = crate::alloc_extern::vec![0u64; words];
        out_copy.as_mut_ptr()
    };
    core::arch::asm!(
        "csrs {port}, {src}",
        "add x0, {dst}, {size}",
        port = const zisk_definitions::SYSCALL_JUMP_DEST_ID,
        size = in(reg) size,
        dst = in(reg) tmp_ptr,
        src = in(reg) bytecode,
        options(nostack, preserves_flags),
    );
    if tmp_ptr as *mut u8 != bitmap {
        core::ptr::copy_nonoverlapping(tmp_ptr as *const u8, bitmap, words * 8);
    }
}
