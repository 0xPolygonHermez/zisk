//! syscall_babyjubjub_add system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::point::SyscallPoint256;

/// Performs the addition of two points on the BabyJubJub (twisted Edwards) curve, storing the
/// result in the first point.
///
/// `BabyJubJubAdd` operates on two affine points `(x, y)`, each coordinate being a 256-bit element
/// of the BN254 scalar field. Each coordinate is represented as an array of four `u64` elements.
/// The syscall takes the addresses of points `p1` and `p2` as two direct operands (no parameter struct).
/// The result of the addition is stored in `p1`.
///
/// The BabyJubJub addition law is complete, so the same formula doubles a point; to double `P`,
/// pass `p1 = p2 = P`.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The caller must ensure that the points `p1` and `p2` are valid points on the BabyJubJub curve.
///
/// The caller must ensure that both `p1` and `p2` coordinates are within the range of the BN254
/// scalar field.
///
/// The resulting point will have both coordinates in the range of the BN254 scalar field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_babyjubjub_add")]
pub extern "C" fn syscall_babyjubjub_add(
    p1: &mut SyscallPoint256,
    p2: &SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BABYJUBJUB_ADD_ID, a: p1, b: p2);
    #[cfg(not(zisk_guest))]
    {
        let p1_coords = [p1.x, p1.y].concat().try_into().unwrap();
        let p2_coords = [p2.x, p2.y].concat().try_into().unwrap();
        let mut p3: [u64; 8] = [0; 8];
        zisk_precomp_helpers::babyjubjub_add(&p1_coords, &p2_coords, &mut p3);
        p1.x.copy_from_slice(&p3[0..4]);
        p1.y.copy_from_slice(&p3[4..8]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p3);
        }
    }
}
