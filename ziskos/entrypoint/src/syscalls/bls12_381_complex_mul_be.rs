//! syscall_bls12_381_complex_mul_be system call interception

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(zisk_guest)]
use core::arch::asm;

use super::bls12_381_complex_mul::SyscallBls12_381ComplexMulParams;

/// Big-endian variant of [`syscall_bls12_381_complex_mul`](super::syscall_bls12_381_complex_mul).
///
/// Identical to `syscall_bls12_381_complex_mul`, except that each 384-bit coordinate
/// of the input/output field elements is expected/produced with its limb order
/// reversed and each limb stored in big-endian byte order.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_bls12_381_complex_mul_be")]
pub extern "C" fn syscall_bls12_381_complex_mul_be(
    params: &mut SyscallBls12_381ComplexMulParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLS12_381_COMPLEX_MUL_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        use super::be_utils::be_swap_6;
        let f1x = be_swap_6(&params.f1.x);
        let f1y = be_swap_6(&params.f1.y);
        let f2x = be_swap_6(&params.f2.x);
        let f2y = be_swap_6(&params.f2.y);
        let f1: [u64; 12] = [f1x, f1y].concat().try_into().unwrap();
        let f2: [u64; 12] = [f2x, f2y].concat().try_into().unwrap();
        let mut f3: [u64; 12] = [0; 12];
        precompiles_helpers::bls12_381_complex_mul(&f1, &f2, &mut f3);
        let x_be = be_swap_6(f3[0..6].try_into().unwrap());
        let y_be = be_swap_6(f3[6..12].try_into().unwrap());
        params.f1.x.copy_from_slice(&x_be);
        params.f1.y.copy_from_slice(&y_be);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(&params.f1.x);
            hints.extend_from_slice(&params.f1.y);
        }
    }
}
