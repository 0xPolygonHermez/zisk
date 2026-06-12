//! syscall_secp256r1_dbl system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::point::SyscallPoint256;

/// Executes the doubling of a point on the Secp256r1 curve.
///
/// `syscall_secp256r1_dbl` operates on a point with two coordinates, each consisting of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements. The syscall takes as a parameter
/// the address of the point, and the result of the doubling operation is stored at the same location.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The caller must ensure that `p` is a finite point on the Secp256r1 curve.
/// The point at infinity is not allowed.
///
/// The caller must ensure that the coordinates of `p` are canonical representatives
/// of elements in the Secp256r1 base field.
///
/// The resulting point will have both coordinates reduced to canonical representatives
/// in the range of the Secp256r1 base field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_secp256r1_dbl")]
pub extern "C" fn syscall_secp256r1_dbl(
    p: &mut SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_SECP256R1_DBL_ID, p);
    #[cfg(not(zisk_guest))]
    {
        let _p1 = [p.x, p.y].concat().try_into().unwrap();
        let mut p3: [u64; 8] = [0; 8];
        precompiles_helpers::secp256r1_dbl(&_p1, &mut p3);
        p.x.copy_from_slice(&p3[0..4]);
        p.y.copy_from_slice(&p3[4..8]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p3);
        }
    }
}
