//! syscall_bls12_381_curve_add_be system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::bls12_381_curve_add::SyscallBls12_381CurveAddParams;

/// Big-endian variant of [`syscall_bls12_381_curve_add`](super::syscall_bls12_381_curve_add).
///
/// Identical to `syscall_bls12_381_curve_add`, except that each 384-bit coordinate of
/// the input/output points is expected/produced with its limb order reversed and
/// each limb stored in big-endian byte order.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bls12_381_curve_add_be")]
pub extern "C" fn syscall_bls12_381_curve_add_be(
    params: &mut SyscallBls12_381CurveAddParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLS12_381_CURVE_ADD_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        use super::be_utils::be_swap_6;
        let p1x = be_swap_6(&params.p1.x);
        let p1y = be_swap_6(&params.p1.y);
        let p2x = be_swap_6(&params.p2.x);
        let p2y = be_swap_6(&params.p2.y);
        let p1: [u64; 12] = [p1x, p1y].concat().try_into().unwrap();
        let p2: [u64; 12] = [p2x, p2y].concat().try_into().unwrap();
        let mut p3: [u64; 12] = [0; 12];
        precompiles_helpers::bls12_381_curve_add(&p1, &p2, &mut p3);
        let x_be = be_swap_6(p3[0..6].try_into().unwrap());
        let y_be = be_swap_6(p3[6..12].try_into().unwrap());
        params.p1.x.copy_from_slice(&x_be);
        params.p1.y.copy_from_slice(&y_be);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&params.p1.x);
            hints.extend_from_slice(&params.p1.y);
        }
    }
}
