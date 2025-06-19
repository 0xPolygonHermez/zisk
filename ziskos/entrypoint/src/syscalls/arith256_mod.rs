//! Arith256Mod system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallArith256ModParams<'a> {
    pub a: &'a [u64; 4],
    pub b: &'a [u64; 4],
    pub c: &'a [u64; 4],
    pub module: &'a [u64; 4],
    pub d: &'a mut [u64; 4],
}

/// Executes the `Arith256Mod` operation, performing a modular 256-bit multiplication and addition:
/// `d = (a * b + c) mod module`.
///
/// The `Arith256Mod` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Arith256Mod`.
///
/// `Arith256Mod` operates on arrays of four `u64` elements. The first parameter is a pointer to a structure
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
pub extern "C" fn syscall_arith256_mod(params: &mut SyscallArith256ModParams) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x802, params);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!()
}
