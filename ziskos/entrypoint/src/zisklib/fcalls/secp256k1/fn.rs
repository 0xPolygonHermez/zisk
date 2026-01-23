use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param, zisklib::FCALL_SECP256K1_FN_INV_ID};
    } else {
        use lib_c::secp256k1_fn_inv_c;
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
pub fn fcall_secp256k1_fn_inv(
    p_value: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let mut result: [u64; 4] = [0; 4];
        secp256k1_fn_inv_c(p_value, &mut result);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p_value, 4);
        ziskos_fcall!(FCALL_SECP256K1_FN_INV_ID);
        [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()]
    }
}

#[allow(unused_variables)]
pub fn fcall_secp256k1_fn_inv_in_place(
    p_value: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let mut result: [u64; 4] = [0; 4];
        secp256k1_fn_inv_c(p_value, &mut result);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p_value, 4);
        ziskos_fcall!(FCALL_SECP256K1_FN_INV_ID);
    }
}
