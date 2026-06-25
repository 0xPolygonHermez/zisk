//! syscall_bls12_381_curve_dbl system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::point::SyscallPoint384;

/// Executes the doubling of a point on the BLS12-381 curve.
///
/// `syscall_bls12_381_curve_dbl` operates on a point with two coordinates, each consisting of 384 bits.
/// Each coordinate is represented as an array of six `u64` elements. The syscall takes as a parameter
/// the address of the point, and the result of the doubling operation is stored at the same location.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The caller must ensure that `p` is a finite point on the BLS12-381 curve.
/// The point at infinity is not allowed.
///
/// The caller must ensure that the coordinates of `p` are canonical representatives
/// of elements in the BLS12-381 base field.
///
/// The resulting point will have both coordinates reduced to canonical representatives
/// in the range of the BLS12-381 base field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bls12_381_curve_dbl")]
pub extern "C" fn syscall_bls12_381_curve_dbl(
    p: &mut SyscallPoint384,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLS12_381_CURVE_DBL_ID, p);
    #[cfg(not(zisk_guest))]
    {
        let _p1 = [p.x, p.y].concat().try_into().unwrap();
        let mut p2: [u64; 12] = [0; 12];
        precompiles_helpers::bls12_381_curve_dbl(&_p1, &mut p2);
        p.x.copy_from_slice(&p2[0..6]);
        p.y.copy_from_slice(&p2[6..12]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p2);
        }
    }
}
