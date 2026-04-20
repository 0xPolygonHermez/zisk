use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_param,
            zisklib::{FCALL_BLS12_381_FP_INV_ID, FCALL_BLS12_381_FP_SQRT_ID}
        };
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::bls12_381::{bls12_381_fp_inv, bls12_381_fp_sqrt};
    }
}

/// Compute the multiplicative inverse of a field element in the base field of the BLS12-381 curve.
///
/// `fcall_bls12_381_fp_inv` operates on a 384-bit field element represented as an array of six `u64` values,
/// and returns the inverse as an array of six `u64` values.
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
pub fn fcall_bls12_381_fp_inv(
    x: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 6] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let result: [u64; 6] = bls12_381_fp_inv(x);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(x, 8);
        ziskos_fcall!(FCALL_BLS12_381_FP_INV_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            [
                ziskos_fcall_get(),
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
            let mut result: MaybeUninit<[u64; 6]> = MaybeUninit::uninit();
            ziskos_inputcpy!(result, 48);
            unsafe { result.assume_init() }
        }
    }
}

/// Compute the square root of a field element in the base field of the BLS12-381 curve, if it exists.
///
/// `fcall_bls12_381_fp_sqrt` operates on a 384-bit field element represented as an array of six `u64` values,
/// and returns an array of seven `u64` values where the first value indicates whether a square root exists (1) or not (0),
/// and the remaining six values represent the square root.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bls12_381_fp_sqrt(
    x: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 7] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let mut result: [u64; 7] = [0; 7];
        bls12_381_fp_sqrt(x, &mut result);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(x, 8);
        ziskos_fcall!(FCALL_BLS12_381_FP_SQRT_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            [
                ziskos_fcall_get(), // results[0] - indicates if a sqrt exists (1) or not (0)
                ziskos_fcall_get(),
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
            let mut result: MaybeUninit<[u64; 7]> = MaybeUninit::uninit();
            ziskos_inputcpy!(result, 56);
            unsafe { result.assume_init() }
        }
    }
}
