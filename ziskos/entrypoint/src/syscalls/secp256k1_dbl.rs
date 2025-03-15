//! Secp256k1Dbl system call interception

#[cfg(target_os = "ziskos")]
use crate::ziskos_syscall;

#[cfg(target_os = "ziskos")]
use core::arch::asm;

use super::point256::SyscallPoint256;

/// Executes the doubling of a point on the Secp256k1 curve.
///
/// The `Secp256k1Dbl` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Secp256k1Dbl`.
///
/// `Secp256k1Dbl` operates on a point with two coordinates, each consisting of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements. The syscall takes as a parameter
/// the address of the point, and the result of the doubling operation is stored at the same location.
///
/// ### Safety
///
/// The caller must ensure that `p1` is a valid pointer to data that is aligned to an eight-byte boundary.

#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_secp256k1_dbl(p1: &mut SyscallPoint256) {
    #[cfg(target_os = "ziskos")]
    ziskos_syscall!(0x804, p1);
    #[cfg(not(target_os = "ziskos"))]
    unreachable!()
}
