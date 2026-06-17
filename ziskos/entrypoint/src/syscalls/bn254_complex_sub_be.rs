//! syscall_bn254_complex_sub_be system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::bn254_complex_sub::SyscallBn254ComplexSubParams;

/// Big-endian variant of [`syscall_bn254_complex_sub`](super::syscall_bn254_complex_sub).
///
/// Identical to `syscall_bn254_complex_sub`, except that each 256-bit coordinate of
/// the input/output field elements is expected/produced with its limb order
/// reversed and each limb stored in big-endian byte order.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bn254_complex_sub_be")]
pub extern "C" fn syscall_bn254_complex_sub_be(
    params: &mut SyscallBn254ComplexSubParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BN254_COMPLEX_SUB_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        use super::be_utils::be_swap_4;
        let f1x = be_swap_4(&params.f1.x);
        let f1y = be_swap_4(&params.f1.y);
        let f2x = be_swap_4(&params.f2.x);
        let f2y = be_swap_4(&params.f2.y);
        let f1: [u64; 8] = [f1x, f1y].concat().try_into().unwrap();
        let f2: [u64; 8] = [f2x, f2y].concat().try_into().unwrap();
        let mut f3: [u64; 8] = [0; 8];
        precompiles_helpers::bn254_complex_sub(&f1, &f2, &mut f3);
        let x_be = be_swap_4(f3[0..4].try_into().unwrap());
        let y_be = be_swap_4(f3[4..8].try_into().unwrap());
        params.f1.x.copy_from_slice(&x_be);
        params.f1.y.copy_from_slice(&y_be);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&params.f1.x);
            hints.extend_from_slice(&params.f1.y);
        }
    }
}
