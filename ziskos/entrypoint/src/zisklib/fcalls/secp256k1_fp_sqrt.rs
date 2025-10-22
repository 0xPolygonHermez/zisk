//! fcall_secp256k1_fp_sqrt free call
use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use crate::FCALL_SECP256K1_FP_SQRT_ID;
    }
}

/// Executes the square root computation over the base field of the `secp256k1` curve.
///
/// Both `fcall_secp256k1_fp_inv` and `fcall2_secp256k1_fp_inv` perform an square root of a 256-bit
/// field element, represented as an array of four `u64` values.
///
/// - `fcall_secp256k1_fp_inv` performs the sqrt and **returns the result directly**.
/// - `fcall2_secp256k1_fp_inv` performs the sqrt but does **not return the result immediately**.
///   You must explicitly retrieve the result using four (4) `fcall_get` instructions.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_secp256k1_fp_sqrt(p_value: &[u64; 4], parity: u64) -> [u64; 5] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p_value, 4);
        ziskos_fcall_param!(parity, 1);
        ziskos_fcall!(FCALL_SECP256K1_FP_SQRT_ID);
        [
            ziskos_fcall_get(), // results[0] - indicates if a sqrt exists (1) or not (0)
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
        ]
    }
}

#[allow(unused_variables)]
pub fn fcall2_secp256k1_fp_sqrt(p_value: &[u64; 4], parity: u64) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p_value, 4);
        ziskos_fcall_param!(parity, 1);
        ziskos_fcall!(FCALL_SECP256K1_FP_SQRT_ID);
    }
}
