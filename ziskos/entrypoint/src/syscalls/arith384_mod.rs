//! Arith384Mod system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallArith384ModParams<'a> {
    pub a: &'a [u64; 6],
    pub b: &'a [u64; 6],
    pub c: &'a [u64; 6],
    pub module: &'a [u64; 6],
    pub d: &'a mut [u64; 6],
}

/// Executes the `Arith384Mod` operation, performing a modular 384-bit multiplication and addition:
/// `d = (a * b + c) mod module`.
///
/// The `Arith384Mod` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Arith384Mod`.
///
/// `Arith384Mod` operates on arrays of four `u64` elements. The first parameter is a pointer to a structure
/// containing five values:
/// - `a`
/// - `b`
/// - `c`
/// - `module`
/// - `d` (the result)
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_arith384_mod(params: &mut SyscallArith384ModParams) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x80B, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!()
}
