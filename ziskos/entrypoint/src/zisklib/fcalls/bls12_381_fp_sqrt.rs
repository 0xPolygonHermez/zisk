//! fcall_bls12_381_fp_sqrt free call
use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use crate::FCALL_BLS12_381_FP_SQRT_ID;
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
pub fn fcall_bls12_381_fp_sqrt(p_value: &[u64; 6]) -> [u64; 7] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
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
