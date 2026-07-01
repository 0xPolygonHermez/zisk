use core::mem::MaybeUninit;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_param,
            zisklib::{FCALL_SECP256K1_FP_INV_ID, FCALL_SECP256K1_FP_SQRT_ID}
        };
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use lib_c::{secp256k1_fp_inv_c};
        use crate::zisklib::fcalls_impl::secp256k1::secp256k1_fp_sqrt;
    }

}

/// Compute the multiplicative inverse of a field element in the base field of the Secp256k1 curve.
///
/// `fcall_secp256k1_fp_inv` operates on a 256-bit field element represented as an array of four `u64` values,
/// and returns the inverse as an array of four `u64` values.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The caller must also ensure that the input value is non-zero.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_secp256k1_fp_inv(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    #[cfg(not(zisk_guest))]
    {
        let mut result: [u64; 4] = [0; 4];
        secp256k1_fp_inv_c(x, &mut result);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(x, 4);
        ziskos_fcall!(FCALL_SECP256K1_FP_INV_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()]
        }
        #[cfg(feature = "inputcpy")]
        {
            let mut res: MaybeUninit<[u64; 4]> = MaybeUninit::uninit();
            ziskos_inputcpy!(res, 32);
            unsafe { res.assume_init() }
        }
    }
}

/// Compute the square root of a field element in the base field of the Secp256k1 curve, if it exists.
///
/// `fcall_secp256k1_fp_sqrt` operates on a 256-bit field element represented as an array of four `u64` values,
/// and returns an array of five `u64` values where the first value indicates whether a square root exists (1) or not (0),
/// and the remaining four values represent the square root.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_secp256k1_fp_sqrt(
    x: &[u64; 4],
    parity: u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 5] {
    #[cfg(not(zisk_guest))]
    {
        let mut result: [u64; 5] = [0; 5];
        secp256k1_fp_sqrt(x, parity, &mut result);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(x, 4);
        ziskos_fcall_param!(parity, 1);
        ziskos_fcall!(FCALL_SECP256K1_FP_SQRT_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            [
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
            ]
        }
        #[cfg(feature = "inputcpy")]
        {
            use core::mem::MaybeUninit;
            let mut res: MaybeUninit<[u64; 5]> = MaybeUninit::uninit();
            ziskos_inputcpy!(res, 40);
            unsafe { res.assume_init() }
        }
    }
}
