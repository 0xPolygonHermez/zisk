//! syscall_bls12_381_curve_dbl system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::point::SyscallPoint384;

/// Executes the doubling of a point on the Bls12_381 curve.
///
/// The `syscall_bls12_381_curve_dbl` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Bls12_381CurveDbl`.
///
/// `syscall_bls12_381_curve_dbl` operates on a point with two coordinates, each consisting of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements. The syscall takes as a parameter
/// the address of the point, and the result of the doubling operation is stored at the same location.
///
/// ### Safety
///
/// The caller must ensure that `p1` is a valid pointer to data that is aligned to an eight-byte boundary.
///
/// The caller must ensure that `p1` coordinates are within the range of the BLS12-381 base field.
///
/// The resulting point will have both coordinates in the range of the BLS12-381 base field.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12_381_curve_dbl(p1: &mut SyscallPoint384) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x80D, p1);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!()
}
