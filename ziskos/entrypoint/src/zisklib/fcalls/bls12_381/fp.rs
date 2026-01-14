use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param,
            zisklib::{FCALL_BLS12_381_FP_INV_ID, FCALL_BLS12_381_FP_SQRT_ID}
        };
    } else {
        use crate::zisklib::fcalls_impl::bls12_381::{bls12_381_fp_inv, bls12_381_fp_sqrt};
    }
}

/// Executes the multiplicative inverse computation over the base field of the `bls12_381` curve.
///
/// `fcall_bls12_381_fp_inv` performs an inversion of a 256-bit field element,
/// represented as an array of four `u64` values.
///
/// - `fcall_bls12_381_fp_inv` performs the inversion and **returns the result directly**.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bls12_381_fp_inv(
    p_value: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 6] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let result: [u64; 6] = bls12_381_fp_inv(p_value);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p_value, 8);
        ziskos_fcall!(FCALL_BLS12_381_FP_INV_ID);
        [
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
        ]
    }
}

/// Executes the multiplicative inverse computation over the base field of the `bls12_381` curve.
///
/// `fcall_bls12_381_fp_sqrt` performs an inversion of a 256-bit field element,
/// represented as an array of four `u64` values.
///
/// - `fcall_bls12_381_fp_sqrt` performs the inversion and **returns the result directly**.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bls12_381_fp_sqrt(
    p_value: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 7] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let mut result: [u64; 7] = [0; 7];
        bls12_381_fp_sqrt(p_value, &mut result);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p_value, 8);
        ziskos_fcall!(FCALL_BLS12_381_FP_SQRT_ID);
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
}
