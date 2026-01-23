use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param, zisklib::FCALL_SECP256K1_ECDSA_VERIFY_ID};
    }
}

/// Hints the ECDSA recovery computation over the `secp256k1` curve.
///
/// Given the public key `PK`, a message hash `z`, and signature components `(r, s)`,
/// this function hints a curve point `P` such that:
///
/// ```text
/// P = [s⁻¹·z (mod n)]G + [s⁻¹·r (mod n)]PK
/// ```
///
/// ### Parameters
///
/// - `pk_value`: The public key `PK = (x, y)`,
///   represented as 8 `u64` limbs in little-endian order: `[x₀, x₁, x₂, x₃, y₀, y₁, y₂, y₃]`
/// - `z_value`: The message hash (prehash), represented as 4 `u64` limbs in little-endian order
/// - `r_value`: The signature `r` component, represented as 4 `u64` limbs in little-endian order
/// - `s_value`: The signature `s` component, represented as 4 `u64` limbs in little-endian order
///
/// ### Returns
///
/// The curve point `P = (x, y)` as 8 `u64` limbs in little-endian order:
/// `[x₀, x₁, x₂, x₃, y₀, y₁, y₂, y₃]`
///
/// ### Safety
///
/// The caller must ensure that all input pointers (`pk_value`, `z_value`, `r_value`, `s_value`) are
/// valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_secp256k1_ecdsa_verify(
    pk_value: &[u64; 8],
    z_value: &[u64; 4],
    r_value: &[u64; 4],
    s_value: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        use crate::zisklib::fcalls_impl;

        // Convert inputs into a single params array
        let mut params: [u64; 20] = [0u64; 20];
        params[0..8].copy_from_slice(pk_value);
        params[8..12].copy_from_slice(z_value);
        params[12..16].copy_from_slice(r_value);
        params[16..20].copy_from_slice(s_value);

        // Call the implementation
        let mut results = [0u64; 8];
        fcalls_impl::secp256k1::fcall_secp256k1_ecdsa_verify(&params, &mut results);

        // Hint the result
        #[cfg(feature = "hints")]
        {
            hints.push(results.len() as u64);
            hints.extend_from_slice(&results);
        }

        results
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(pk_value, 8);
        ziskos_fcall_param!(z_value, 4);
        ziskos_fcall_param!(r_value, 4);
        ziskos_fcall_param!(s_value, 4);
        ziskos_fcall!(FCALL_SECP256K1_ECDSA_VERIFY_ID);
        [
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
}
