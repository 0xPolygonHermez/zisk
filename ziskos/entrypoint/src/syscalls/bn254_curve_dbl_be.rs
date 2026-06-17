//! syscall_bn254_curve_dbl_be system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::point::SyscallPoint256;

/// Big-endian variant of [`syscall_bn254_curve_dbl`](super::syscall_bn254_curve_dbl).
///
/// Identical to `syscall_bn254_curve_dbl`, except that each 256-bit coordinate of
/// the point is expected/produced with its limb order reversed and each limb
/// stored in big-endian byte order.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bn254_curve_dbl_be")]
pub extern "C" fn syscall_bn254_curve_dbl_be(
    p: &mut SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BN254_CURVE_DBL_BE_ID, p);
    #[cfg(not(zisk_guest))]
    {
        use super::be_utils::be_swap_4;
        let px = be_swap_4(&p.x);
        let py = be_swap_4(&p.y);
        let p1: [u64; 8] = [px, py].concat().try_into().unwrap();
        let mut p2: [u64; 8] = [0; 8];
        precompiles_helpers::bn254_curve_dbl(&p1, &mut p2);
        let x_be = be_swap_4(p2[0..4].try_into().unwrap());
        let y_be = be_swap_4(p2[4..8].try_into().unwrap());
        p.x.copy_from_slice(&x_be);
        p.y.copy_from_slice(&y_be);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&p.x);
            hints.extend_from_slice(&p.y);
        }
    }
}
