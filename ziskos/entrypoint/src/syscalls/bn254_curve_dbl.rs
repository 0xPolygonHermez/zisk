//! syscall_bn254_curve_dbl system call interception

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::ziskos_syscall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

use super::point::SyscallPoint256;

/// Executes the doubling of a point on the BN254 curve.
///
/// `syscall_bn254_curve_dbl` operates on a point with two coordinates, each consisting of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements. The syscall takes as a parameter
/// the address of the point, and the result of the doubling operation is stored at the same location.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The caller must ensure that `p` is a valid point on the BN254 curve.
///
/// The caller must ensure that `p` coordinates are within the range of the BN254 base field.
///
/// The resulting point will have both coordinates in the range of the BN254 base field.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bn254_curve_dbl")]
pub extern "C" fn syscall_bn254_curve_dbl(
    p: &mut SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    ziskos_syscall!(zisk_definitions::SYSCALL_BN254_CURVE_DBL_ID, p);
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let _p1 = [p.x, p.y].concat().try_into().unwrap();
        let mut p2: [u64; 8] = [0; 8];
        precompiles_helpers::bn254_curve_dbl(&_p1, &mut p2);
        p.x.copy_from_slice(&p2[0..4]);
        p.y.copy_from_slice(&p2[4..8]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p2);
        }
    }
}
