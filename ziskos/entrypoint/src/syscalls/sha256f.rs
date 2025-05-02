//! Sha256 system call interception

#[cfg(target_os = "ziskos")]
use core::arch::asm;

#[cfg(target_os = "ziskos")]
use crate::ziskos_syscall;

/// Executes the Sha256 permutation on the given state.
///
/// The `Sha256` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Sha256`.
///
/// The syscall takes as a parameter the address of a state data (512 bits = 64 bytes)
/// and the result of the sha256f operation is stored at the same location
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_sha256_f(state: *mut [u64; 4], input: *const [u64; 8]) {
    #[cfg(target_os = "ziskos")]
    ziskos_syscall!(0x805, state);
    #[cfg(not(target_os = "ziskos"))]
    unreachable!()
}
