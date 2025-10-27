//! Add256 system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall_ret_u64;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallAdd256Params<'a> {
    pub a: &'a [u64; 4],
    pub b: &'a [u64; 4],
    pub cin: u64,
    pub c: &'a mut [u64; 4],
}

/// Executes the `Add256` operation, performing a 256-bit addition:
/// `a + b + cin = cout | c`.
///
/// The `Add256` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Add256`.
///
/// `Add256` operates on arrays of four `u64` elements. The first parameter is a pointer to a structure
/// containing four values: `a`, `b`, `cin`, and the result `c`:
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_add256(params: &mut SyscallAdd256Params) -> u64 {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall_ret_u64!(0x811, params)
}
