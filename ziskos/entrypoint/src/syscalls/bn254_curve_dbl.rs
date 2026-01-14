//! syscall_bn254_curve_dbl system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::point::SyscallPoint256;

/// Executes the doubling of a point on the Bn254 curve.
///
/// The `syscall_bn254_curve_dbl` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Bn254CurveDbl`.
///
/// `syscall_bn254_curve_dbl` operates on a point with two coordinates, each consisting of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements. The syscall takes as a parameter
/// the address of the point, and the result of the doubling operation is stored at the same location.
///
/// ### Safety
///
/// The caller must ensure that `p1` is a valid pointer to data that is aligned to an eight-byte boundary.
///
/// The caller must ensure that `p1` coordinates are within the range of the BN254 base field.
///
/// The resulting point will have both coordinates in the range of the BN254 base field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bn254_curve_dbl")]
pub extern "C" fn syscall_bn254_curve_dbl(
    p1: &mut SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(0x807, p1);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let _p1 = [p1.x, p1.y].concat().try_into().unwrap();
        let mut p2: [u64; 8] = [0; 8];
        precompiles_helpers::bn254_curve_dbl(&_p1, &mut p2);
        p1.x.copy_from_slice(&p2[0..4]);
        p1.y.copy_from_slice(&p2[4..8]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p2);
        }
    }
}
