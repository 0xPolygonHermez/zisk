/// Copies free-input data, as fcall result, directly to a memory location.
///
/// This macro writes free-input data to the specified pointer using
/// custom CSR instructions. The memory does not need to be initialized.
///
/// # Arguments
/// * `$dest` - Mutable reference to the destination (array, slice, or MaybeUninit)
/// * `$size` - Size in bytes (must be a const literal)
///
/// # Safety
/// The caller must ensure the destination is valid and properly aligned.

#[macro_export]
#[cfg(zisk_guest)]
macro_rules! ziskos_inputcpy {
    ($dest:expr, $size:literal) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {ptr}",
                "addi x0, {ptr}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_INPUTCPY_ID,
                size = const $size,
                ptr = in(reg) $dest.as_mut_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
    ($dest:expr, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {ptr}",
                "add x0, {ptr}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_INPUTCPY_ID,
                size = in(reg) $size,
                ptr = in(reg) $dest.as_mut_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
}

/// Copies memory from source to destination using DMA operations.
///
/// This macro performs a memory copy operation using custom CSR instructions
/// for optimized performance in the zkVM environment.
///
/// # Arguments
/// * `$dst` - Mutable reference to the destination (array, slice, or MaybeUninit)
/// * `$src` - Reference to the source (array or slice)
/// * `$size` - Size in bytes (can be a literal or expression)
///
/// # Safety
/// The caller must ensure both source and destination are valid and properly aligned,
/// and that they do not overlap in memory.

#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_memcpy {
    ($dst:expr, $src: expr, $size:literal) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "addi x0, {dst}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_MEMCPY_ID,
                size = const $size,
                dst = in(reg) $dst.as_mut_ptr(),
                src = in(reg) $src.as_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
    ($dst:expr, $src: expr, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "add x0, {dst}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_MEMCPY_ID,
                size = in(reg) $size,
                dst = in(reg) $dst.as_mut_ptr(),
                src = in(reg) $src.as_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
    (dst_ptr: $dst:expr, $src:expr, $size:literal) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "addi x0, {dst}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_MEMCPY_ID,
                size = const $size,
                dst = in(reg) $dst,
                src = in(reg) $src.as_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
    (ptr: $dst:expr, $src:expr, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "add x0, {dst}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_MEMCPY_ID,
                size = in(reg) $size,
                dst = in(reg) $dst,      // it is already a *mut u8, sin as_mut_ptr()
                src = in(reg) $src,      // it is already a *mut u8, sin as_ptr()
                options(nostack, preserves_flags),
            );
        }
    }};
}

/// Compares two memory regions for equality using DMA operations.
///
/// This macro performs a memory comparison operation using custom CSR instructions
/// for optimized performance in the zkVM environment. The result is stored in a register.
///
/// # Arguments
/// * `$dst` - Mutable reference to the first memory region (array or slice)
/// * `$src` - Reference to the second memory region (array or slice)
/// * `$size` - Size in bytes to compare (can be a literal or expression)
///
/// # Safety
/// The caller must ensure both memory regions are valid and properly aligned.

#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_memcmp {
    ($dst:expr, $src: expr, $size:literal) => {{
        let v: i64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, {src}",
                "addi x0, {dst}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_MEMCMP_ID,
                size = const $size,
                dst = in(reg) $dst.as_ptr(),
                src = in(reg) $src.as_ptr(),
                res = out(reg) v,
                options(nostack, preserves_flags),
            );
        }
        v
    }};
    ($dst:expr, $src: expr, $size:expr) => {{
        let v: i64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, {src}",
                "add x0, {dst}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_MEMCMP_ID,
                size = in(reg) $size,
                dst = in(reg) $dst.as_ptr(),
                src = in(reg) $src.as_ptr(),
                res = out(reg) v,
                options(nostack, preserves_flags),
            );
        }
        v
    }};
}

/// Fills a memory region with a constant byte value using DMA operations.
///
/// This macro performs a memory set operation using custom CSR instructions
/// for optimized performance in the zkVM environment.
///
/// # Arguments
/// * `$dst` - Mutable reference to the destination memory (array, slice, or MaybeUninit)
/// * `$value` - Byte value to fill (can be a literal or expression)
/// * `$size` - Size in bytes (can be a literal or expression)
///
/// # Safety
/// The caller must ensure the destination is valid and properly aligned.

#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_memset {
    ($dst:expr, $value: literal, $size:literal) => {{
        unsafe {
            core::arch::asm!(
                "csrsi {port}, 2",
                "addi x0, {dst}, {size}",
                "addi x0, {dst}, {value}",
                port = const zisk_definitions::SYSCALL_DMA_MEMSET_ID,
                size = const $size,
                value = const $value,
                dst = in(reg) $dst.as_mut_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
    ($dst:expr, $value: literal, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {dst}",
                "addi x0, {size}, {value}",
                port = const zisk_definitions::SYSCALL_DMA_MEMSET_ID,
                size = in(reg) $size,
                value = const $value,
                dst = in(reg) $dst.as_mut_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
    (ptr: $dst:expr, $value: literal, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {dst}",
                "addi x0, {size}, {value}",
                port = const zisk_definitions::SYSCALL_DMA_MEMSET_ID,
                size = in(reg) $size,
                value = const $value,
                dst = in(reg) $dst,
                options(nostack, preserves_flags),
            );
        }
    }};
    ($dst:expr, $value: expr, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "call memset",
                in("a0") $dst.as_mut_ptr(),
                in("a1") $value,
                in("a2") $size,
                lateout("t0") _,
                lateout("a1") _,
                lateout("ra") _,
                options(nostack, preserves_flags),
            );
        }
    }};
}

// Re-export the temporal-reference/advice ABI constants from zisk_definitions so that the macros
// below expand correctly in guest code, which does not depend on zisk_definitions itself. Mirrors
// what `profile.rs` does for the profiling macros.
#[cfg(zisk_guest)]
pub use zisk_definitions::{
    EXECUTE_ADVICE_MARKER_ID, SYSCALL_DMA_MTCMP_ID, SYSCALL_DMA_MTCPY_ID, SYSCALL_TEMPORAL_REF_ID,
};

/// Requests a *temporal reference*: an opaque handle to the current point of the execution, to be
/// handed later to [`ziskos_mtcpy!`] / [`ziskos_mtcmp!`] so they read their source as it was here.
///
/// On its own a temporal reference grants nothing: the emulator keeps no memory history, so the
/// regions that must be readable back have to be announced with [`ziskos_execute_advice!`] while
/// the reference is still the most recent one. Prefer [`ziskos_temporal_snapshot!`], which does
/// both in one go and cannot be split apart by the compiler.
///
/// # Returns
/// The temporal reference, as a `u64`.

#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_temporal_ref {
    () => {{
        let tref: u64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, x0",
                port = const $crate::SYSCALL_TEMPORAL_REF_ID,
                res = out(reg) tref,
                options(nostack, preserves_flags),
            );
        }
        tref
    }};
}

/// Tells the emulator to keep a copy of a memory region as it is now, bound to the temporal
/// reference most recently requested with [`ziskos_temporal_ref!`], so that the `mt` DMA
/// operations can read the region back after it has been overwritten.
///
/// Must be reached while that reference is still the most recent one; a function call in between
/// invalidates it. Use [`ziskos_temporal_snapshot!`] unless you need to advise several regions for
/// the same reference, in which case chain the extra calls right after the snapshot.
///
/// # Arguments
/// * `$src` - Reference to the region to capture (array or slice), or a raw pointer with `ptr:`
/// * `$size` - Size in bytes (a literal keeps the count out of a register)
///
/// # Safety
/// The caller must ensure the region is valid and readable for `$size` bytes.

#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_execute_advice {
    ($src:expr, $size:literal) => {{
        unsafe {
            core::arch::asm!(
                "addi x0, x0, {marker}",
                "addi x0, {ptr}, {size}",
                "addi x0, x0, {marker}",
                marker = const $crate::EXECUTE_ADVICE_MARKER_ID,
                size = const $size,
                ptr = in(reg) $src.as_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
    ($src:expr, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "addi x0, x0, {marker}",
                "add x0, {ptr}, {size}",
                "addi x0, x0, {marker}",
                marker = const $crate::EXECUTE_ADVICE_MARKER_ID,
                size = in(reg) $size,
                ptr = in(reg) $src.as_ptr(),
                options(nostack, preserves_flags),
            );
        }
    }};
    (ptr: $src:expr, $size:literal) => {{
        unsafe {
            core::arch::asm!(
                "addi x0, x0, {marker}",
                "addi x0, {ptr}, {size}",
                "addi x0, x0, {marker}",
                marker = const $crate::EXECUTE_ADVICE_MARKER_ID,
                size = const $size,
                ptr = in(reg) $src,
                options(nostack, preserves_flags),
            );
        }
    }};
    (ptr: $src:expr, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "addi x0, x0, {marker}",
                "add x0, {ptr}, {size}",
                "addi x0, x0, {marker}",
                marker = const $crate::EXECUTE_ADVICE_MARKER_ID,
                size = in(reg) $size,
                ptr = in(reg) $src,
                options(nostack, preserves_flags),
            );
        }
    }};
}

/// Requests a temporal reference and captures a region at it, in a single indivisible sequence.
///
/// This is the form to reach for: the reference and the region it binds cannot drift apart, which
/// is the one way of getting the `mt` operations wrong.
///
/// # Arguments
/// * `$src` - Reference to the region to capture (array or slice), or a raw pointer with `ptr:`
/// * `$size` - Size in bytes (a literal keeps the count out of a register)
///
/// # Returns
/// The temporal reference the region was captured at, as a `u64`.
///
/// # Safety
/// The caller must ensure the region is valid and readable for `$size` bytes.

#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_temporal_snapshot {
    ($src:expr, $size:literal) => {{
        let tref: u64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, x0",
                "addi x0, x0, {marker}",
                "addi x0, {ptr}, {size}",
                "addi x0, x0, {marker}",
                port = const $crate::SYSCALL_TEMPORAL_REF_ID,
                marker = const $crate::EXECUTE_ADVICE_MARKER_ID,
                size = const $size,
                ptr = in(reg) $src.as_ptr(),
                res = out(reg) tref,
                options(nostack, preserves_flags),
            );
        }
        tref
    }};
    ($src:expr, $size:expr) => {{
        let tref: u64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, x0",
                "addi x0, x0, {marker}",
                "add x0, {ptr}, {size}",
                "addi x0, x0, {marker}",
                port = const $crate::SYSCALL_TEMPORAL_REF_ID,
                marker = const $crate::EXECUTE_ADVICE_MARKER_ID,
                size = in(reg) $size,
                ptr = in(reg) $src.as_ptr(),
                res = out(reg) tref,
                options(nostack, preserves_flags),
            );
        }
        tref
    }};
    (ptr: $src:expr, $size:literal) => {{
        let tref: u64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, x0",
                "addi x0, x0, {marker}",
                "addi x0, {ptr}, {size}",
                "addi x0, x0, {marker}",
                port = const $crate::SYSCALL_TEMPORAL_REF_ID,
                marker = const $crate::EXECUTE_ADVICE_MARKER_ID,
                size = const $size,
                ptr = in(reg) $src,
                res = out(reg) tref,
                options(nostack, preserves_flags),
            );
        }
        tref
    }};
    (ptr: $src:expr, $size:expr) => {{
        let tref: u64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, x0",
                "addi x0, x0, {marker}",
                "add x0, {ptr}, {size}",
                "addi x0, x0, {marker}",
                port = const $crate::SYSCALL_TEMPORAL_REF_ID,
                marker = const $crate::EXECUTE_ADVICE_MARKER_ID,
                size = in(reg) $size,
                ptr = in(reg) $src,
                res = out(reg) tref,
                options(nostack, preserves_flags),
            );
        }
        tref
    }};
}

/// Copies memory from source to destination using DMA operations, reading the source as it was at
/// a temporal reference rather than as it is now.
///
/// The source range must have been captured for `$tref` beforehand — see
/// [`ziskos_temporal_snapshot!`]. Reading a range that was never advised, or one whose temporal
/// reference has already been evicted, aborts the execution.
///
/// # Arguments
/// * `$dst` - Mutable reference to the destination (array, slice, or MaybeUninit)
/// * `$src` - Reference to the source (array or slice)
/// * `$size` - Size in bytes (a literal keeps the count out of a register)
/// * `$tref` - Temporal reference the source is read at
///
/// # Safety
/// The caller must ensure both source and destination are valid and properly aligned, and that
/// they do not overlap in memory.

#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_mtcpy {
    ($dst:expr, $src:expr, $size:literal, $tref:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "addi x0, {dst}, {size}",
                "add x0, {tref}, x0",
                port = const $crate::SYSCALL_DMA_MTCPY_ID,
                size = const $size,
                dst = in(reg) $dst.as_mut_ptr(),
                src = in(reg) $src.as_ptr(),
                tref = in(reg) $tref,
                options(nostack, preserves_flags),
            );
        }
    }};
    ($dst:expr, $src:expr, $size:expr, $tref:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "add x0, {dst}, {size}",
                "add x0, {tref}, x0",
                port = const $crate::SYSCALL_DMA_MTCPY_ID,
                size = in(reg) $size,
                dst = in(reg) $dst.as_mut_ptr(),
                src = in(reg) $src.as_ptr(),
                tref = in(reg) $tref,
                options(nostack, preserves_flags),
            );
        }
    }};
    (ptr: $dst:expr, $src:expr, $size:expr, $tref:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "add x0, {dst}, {size}",
                "add x0, {tref}, x0",
                port = const $crate::SYSCALL_DMA_MTCPY_ID,
                size = in(reg) $size,
                dst = in(reg) $dst,      // it is already a *mut u8, sin as_mut_ptr()
                src = in(reg) $src,      // it is already a *mut u8, sin as_ptr()
                tref = in(reg) $tref,
                options(nostack, preserves_flags),
            );
        }
    }};
}

/// Compares two memory regions using DMA operations, reading the second one as it was at a
/// temporal reference rather than as it is now.
///
/// The source range must have been captured for `$tref` beforehand — see
/// [`ziskos_temporal_snapshot!`]. Reading a range that was never advised, or one whose temporal
/// reference has already been evicted, aborts the execution.
///
/// # Arguments
/// * `$dst` - Reference to the first memory region (array or slice), read as it is now
/// * `$src` - Reference to the second memory region (array or slice), read at `$tref`
/// * `$size` - Size in bytes to compare (a literal keeps the count out of a register)
/// * `$tref` - Temporal reference the second region is read at
///
/// # Returns
/// The same `i64` as [`ziskos_memcmp!`]: zero when the regions match, otherwise the signed
/// difference of the first pair of bytes that differ.
///
/// # Safety
/// The caller must ensure both memory regions are valid and properly aligned.

#[cfg(zisk_guest)]
#[macro_export]
macro_rules! ziskos_mtcmp {
    ($dst:expr, $src:expr, $size:literal, $tref:expr) => {{
        let v: i64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, {src}",
                "addi x0, {dst}, {size}",
                "add x0, {tref}, x0",
                port = const $crate::SYSCALL_DMA_MTCMP_ID,
                size = const $size,
                dst = in(reg) $dst.as_ptr(),
                src = in(reg) $src.as_ptr(),
                tref = in(reg) $tref,
                res = out(reg) v,
                options(nostack, preserves_flags),
            );
        }
        v
    }};
    ($dst:expr, $src:expr, $size:expr, $tref:expr) => {{
        let v: i64;
        unsafe {
            core::arch::asm!(
                "csrrs {res}, {port}, {src}",
                "add x0, {dst}, {size}",
                "add x0, {tref}, x0",
                port = const $crate::SYSCALL_DMA_MTCMP_ID,
                size = in(reg) $size,
                dst = in(reg) $dst.as_ptr(),
                src = in(reg) $src.as_ptr(),
                tref = in(reg) $tref,
                res = out(reg) v,
                options(nostack, preserves_flags),
            );
        }
        v
    }};
}
