//! fcall_secp256k1_fn_inv free call
use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(target_os = "ziskos")] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get};
        use crate::FCALL_SECP256K1_FN_INV_ID;
    }
}
/// Executes the multiplicative inverse computation over the scalar field of the `secp256k1` curve.
///
/// Both `fcall_secp256k1_fn_inv` and `fcall2_secp256k1_fn_inv` perform an inversion of a 256-bit
/// scalar field element, represented as an array of four `u64` values.
///
/// - `fcall_secp256k1_fn_inv` performs the inversion and **returns the result directly**.
/// - `fcall2_secp256k1_fn_inv` performs the inversion but does **not return the result immediately**.
///   You must explicitly retrieve the result using four (4) `fcall_get` instructions.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_secp256k1_fn_inv(p_value: &[u64; 4]) -> [u64; 4] {
    #[cfg(not(target_os = "ziskos"))]
    unreachable!();
    #[cfg(target_os = "ziskos")]
    {
        ziskos_fcall!(FCALL_SECP256K1_FN_INV_ID, p_value);
        [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()]
    }
}

#[allow(unused_variables)]
pub fn fcall2_secp256k1_fn_inv(p_value: &[u64; 4]) {
    #[cfg(not(target_os = "ziskos"))]
    unreachable!();
    #[cfg(target_os = "ziskos")]
    {
        ziskos_fcall!(FCALL_SECP256K1_FN_INV_ID, p_value);
    }
}
