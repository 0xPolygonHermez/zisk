//! Secp256r1Add (big-endian) system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::secp256r1_add::SyscallSecp256r1AddParams;

/// Big-endian variant of [`syscall_secp256r1_add`](super::syscall_secp256r1_add).
///
/// Identical to `syscall_secp256r1_add`, except that each 256-bit coordinate of
/// the input/output points is expected/produced with its limb order reversed and
/// each limb stored in big-endian byte order.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_secp256r1_add_be")]
pub extern "C" fn syscall_secp256r1_add_be(
    params: &mut SyscallSecp256r1AddParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_SECP256R1_ADD_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        use super::be_utils::be_swap_4;
        let p1x = be_swap_4(&params.p1.x);
        let p1y = be_swap_4(&params.p1.y);
        let p2x = be_swap_4(&params.p2.x);
        let p2y = be_swap_4(&params.p2.y);
        let p1: [u64; 8] = [p1x, p1y].concat().try_into().unwrap();
        let p2: [u64; 8] = [p2x, p2y].concat().try_into().unwrap();
        let mut p3: [u64; 8] = [0; 8];
        precompiles_helpers::secp256r1_add(&p1, &p2, &mut p3);
        let x_be = be_swap_4(p3[0..4].try_into().unwrap());
        let y_be = be_swap_4(p3[4..8].try_into().unwrap());
        params.p1.x.copy_from_slice(&x_be);
        params.p1.y.copy_from_slice(&y_be);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&params.p1.x);
            hints.extend_from_slice(&params.p1.y);
        }
    }
}
