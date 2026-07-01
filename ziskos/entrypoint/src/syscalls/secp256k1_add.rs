//! Secp256k1Add system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::point::SyscallPoint256;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallSecp256k1AddParams<'a> {
    pub p1: &'a mut SyscallPoint256,
    pub p2: &'a SyscallPoint256,
}

/// Performs the addition of two points on the Secp256k1 curve, storing the result in the first point.
///
/// `Secp256k1Add` operates on two points, each with two coordinates of 256 bits.
/// Each coordinate is represented as an array of four `u64` elements.
/// The syscall takes as a parameter the address of a structure containing points `p1` and `p2`.
/// The result of the addition is stored in `p1`.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The caller must ensure that both `p1` and `p2` are finite points on the Secp256k1 curve.
/// The point at infinity is not allowed.
///
/// The caller must ensure that `p1` is neither equal to `p2` nor the negation of `p2`
/// (i.e. `p1 ≠ p2` and `p1 ≠ -p2`).
///
/// The caller must ensure that the coordinates of `p1` and `p2` are canonical representatives
/// of elements in the Secp256k1 base field.
///
/// The resulting point will have both coordinates reduced to canonical representatives
/// in the range of the Secp256k1 base field.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_secp256k1_add")]
pub extern "C" fn syscall_secp256k1_add(
    params: &mut SyscallSecp256k1AddParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_SECP256K1_ADD_ID, params);
    #[cfg(not(zisk_guest))]
    {
        let p1 = [params.p1.x, params.p1.y].concat().try_into().unwrap();
        let p2 = [params.p2.x, params.p2.y].concat().try_into().unwrap();
        let mut p3: [u64; 8] = [0; 8];
        precompiles_helpers::secp256k1_add(&p1, &p2, &mut p3);
        params.p1.x.copy_from_slice(&p3[0..4]);
        params.p1.y.copy_from_slice(&p3[4..8]);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p3);
        }
    }
}
