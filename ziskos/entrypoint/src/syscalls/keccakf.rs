//! Keccak system call interception

#[cfg(target_os = "ziskos")]
use core::arch::asm;

#[cfg(target_os = "ziskos")]
use crate::ziskos_syscall;

/// Executes the Keccak256 permutation on the given state.
///
/// The `Keccak` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Keccak`.
///
/// The syscall takes as a parameter the address of a state data (1600 bits = 200 bytes)
/// and the result of the keccakf operation is stored at the same location
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_keccak_f(state: *mut [u64; 25]) {
    #[cfg(target_os = "ziskos")]
    ziskos_syscall!(0x800, state);
    #[cfg(not(target_os = "ziskos"))]
    unreachable!()
}
