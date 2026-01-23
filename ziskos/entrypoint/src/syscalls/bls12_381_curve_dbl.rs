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
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bls12_381_curve_dbl")]
pub extern "C" fn syscall_bls12_381_curve_dbl(
    p1: &mut SyscallPoint384,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x80D, p1);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let _p1 = [p1.x, p1.y].concat().try_into().unwrap();
        let mut p2: [u64; 12] = [0; 12];
        precompiles_helpers::bls12_381_curve_dbl(&_p1, &mut p2);
        p1.x.copy_from_slice(&p2[0..6]);
        p1.y.copy_from_slice(&p2[6..12]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p2);
        }
    }
}
