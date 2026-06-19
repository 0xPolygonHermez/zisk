use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_param,
            zisklib::{FCALL_BLS12_381_FP2_INV_ID, FCALL_BLS12_381_FP2_SQRT_ID}
        };
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::bls12_381::{bls12_381_fp2_inv, bls12_381_fp2_sqrt_13};
    }

}

/// Compute the multiplicative inverse of a field element in the complex extension field of the BLS12-381 curve.
///
/// `fcall_bls12_381_fp2_inv` operates on a 384-bit field element represented as an array of twelve `u64` values,
/// and returns the inverse as an array of twelve `u64` values.
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
pub fn fcall_bls12_381_fp2_inv(
    x: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    #[cfg(not(zisk_guest))]
    {
        let result: [u64; 12] = bls12_381_fp2_inv(x);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(x, 12);
        ziskos_fcall!(FCALL_BLS12_381_FP2_INV_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            [
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
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
            let mut res: MaybeUninit<[u64; 12]> = MaybeUninit::uninit();
            ziskos_inputcpy!(res, 96);
            unsafe { res.assume_init() }
        }
    }
}

/// Compute the square root of a field element in the complex extension field of the BLS12-381 curve, if it exists.
///
/// `fcall_bls12_381_fp2_sqrt` operates on a 384-bit field element represented as an array of twelve `u64` values,
/// and returns an array of thirteen `u64` values where the first value indicates whether a square root exists (1) or not (0),
/// and the remaining twelve values represent the square root.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bls12_381_fp2_sqrt(
    x: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 13] {
    #[cfg(not(zisk_guest))]
    {
        let result: [u64; 13] = bls12_381_fp2_sqrt_13(x);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(x, 16);
        ziskos_fcall!(FCALL_BLS12_381_FP2_SQRT_ID);
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
            let mut res: MaybeUninit<[u64; 13]> = MaybeUninit::uninit();
            ziskos_inputcpy!(res, 104);
            unsafe { res.assume_init() }
        }
    }
}
