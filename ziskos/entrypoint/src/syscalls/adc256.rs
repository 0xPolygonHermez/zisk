//! Adc256 system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallAdc256Params<'a> {
    pub a: &'a [u64; 4],
    pub b: &'a [u64; 4],
    pub dl: &'a mut [u64; 4],
    pub dh: &'a mut u64, // 1 or 0
}

/// Executes the `Adc256` operation, performing a 256-bit addition:
/// `a + b + 1 = dh | dl`.
///
/// The `Adc256` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Adc256`.
///
/// `Adc256` operates on arrays of four `u64` elements. The first parameter is a pointer to a structure
/// containing five values: `a`, `b`, `c`, and the result, two 256-bit chunks for `d`:
/// - `dh`: The most significant 256-bit chunk.
/// - `dl`: The least significant 256-bit chunk.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_adc256(params: &mut SyscallAdc256Params) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x812, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!()
}
