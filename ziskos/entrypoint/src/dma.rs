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
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
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

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
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
    (ptr: $dst:expr, $src:expr, $size:expr) => {{
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "add x0, {dst}, {size}",
                port = const zisk_definitions::SYSCALL_DMA_MEMCPY_ID,
                size = in(reg) $size,
                dst = in(reg) $dst,      // ya es *mut u8, sin as_mut_ptr()
                src = in(reg) $src,      // ya es *mut u8, sin as_ptr()
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

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[macro_export]
macro_rules! ziskos_memcmp {
    ($dst:expr, $src: expr, $size:literal) => {{
        let v: i64;
        unsafe {
            core::arch::asm!(
                "csrs {port}, {src}",
                "addi {res}, {dst}, {size}",
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
                "csrs {port}, {src}",
                "add {res}, {dst}, {size}",
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

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
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
