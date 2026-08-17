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
/// * `bitmap` and `bytecode` are 8-byte aligned. The machine reads and writes whole
///   aligned 64-bit words; an unaligned run is not arithmetizable. Check it and use
///   a software walk otherwise.
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
    ($bitmap:expr, $bytecode:expr, $size:expr) => {{
        if $size != 0 {
            unsafe {
                core::arch::asm!(
                    "csrs {port}, {src}",
                    "add x0, {dst}, {size}",
                    port = const zisk_definitions::SYSCALL_JUMP_DEST_ID,
                    size = in(reg) $size,
                    dst = in(reg) $bitmap.as_mut_ptr(),
                    src = in(reg) $bytecode.as_ptr(),
                    options(nostack, preserves_flags),
                );
            }
        }
    }};
    (dst_ptr: $bitmap:expr, $bytecode:expr, $size:expr) => {{
        if $size != 0 {
            unsafe {
                core::arch::asm!(
                    "csrs {port}, {src}",
                    "add x0, {dst}, {size}",
                    port = const zisk_definitions::SYSCALL_JUMP_DEST_ID,
                    size = in(reg) $size,
                    dst = in(reg) $bitmap,
                    src = in(reg) $bytecode.as_ptr(),
                    options(nostack, preserves_flags),
                );
            }
        }
    }};
    (ptr: $bitmap:expr, $bytecode:expr, $size:expr) => {{
        if $size != 0 {
            unsafe {
                core::arch::asm!(
                    "csrs {port}, {src}",
                    "add x0, {dst}, {size}",
                    port = const zisk_definitions::SYSCALL_JUMP_DEST_ID,
                    size = in(reg) $size,
                    dst = in(reg) $bitmap,
                    src = in(reg) $bytecode,
                    options(nostack, preserves_flags),
                );
            }
        }
    }};
}
