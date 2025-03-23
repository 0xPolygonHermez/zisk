//! Keccak system call interception

#[cfg(target_os = "ziskos")]
use core::arch::asm;

#[cfg(target_os = "ziskos")]
use crate::ziskos_syscall;
/// Executes the Keccak256 permutation on the given state.
///
/// The keccak system call execute CSR set on custom port, when transpiling from riscv to zisk
/// replace this instruction with precompiled operation, in this case keccak permutation.
/// The address with the input state data (1600 bits = 200 bytes) is the value "set" to
/// the CSR register, this address is store in one register, no always the same.
///
/// ### Safety
///
/// The caller must ensure that `state` is valid pointer to data that is aligned along a eigth
/// byte boundary.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_keccak_f(state: *mut [u64; 25]) {
    #[cfg(target_os = "ziskos")]
    ziskos_syscall!(0x800, state);
    #[cfg(not(target_os = "ziskos"))]
    unreachable!()
}
