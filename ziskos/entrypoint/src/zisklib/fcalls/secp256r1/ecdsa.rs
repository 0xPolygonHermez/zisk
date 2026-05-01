use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_param, zisklib::FCALL_SECP256R1_ECDSA_VERIFY_ID};
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    }
}

/// Hints the ECDSA recovery computation over the Secp256r1 curve.
///
/// Given the public key `PK`, a message hash `z`, and signature components `(r, s)`,
/// this function hints a curve point `P` such that:
/// P = [s⁻¹·z (mod n)]G + [s⁻¹·r (mod n)]PK
///
/// ### Parameters
///
/// - `pk`: The public key `PK = (x, y)`,
///   represented as 8 `u64` limbs in little-endian order: `[x₀, x₁, x₂, x₃, y₀, y₁, y₂, y₃]`
/// - `z`: The message hash (prehash), represented as 4 `u64` limbs in little-endian order
/// - `r`: The signature `r` component, represented as 4 `u64` limbs in little-endian order
/// - `s`: The signature `s` component, represented as 4 `u64` limbs in little-endian order
///
/// ### Returns
///
/// The curve point `P = (x, y)` as 8 `u64` limbs in little-endian order:
/// `[x₀, x₁, x₂, x₃, y₀, y₁, y₂, y₃]`
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The caller must ensure that s is non-zero.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_secp256r1_ecdsa_verify(
    pk: &[u64; 8],
    z: &[u64; 4],
    r: &[u64; 4],
    s: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        use crate::zisklib::fcalls_impl;

        // Convert inputs into a single params array
        let mut params: [u64; 20] = [0u64; 20];
        params[0..8].copy_from_slice(pk);
        params[8..12].copy_from_slice(z);
        params[12..16].copy_from_slice(r);
        params[16..20].copy_from_slice(s);

        // Call the implementation
        let mut results = [0u64; 8];
        fcalls_impl::secp256r1::fcall_secp256r1_ecdsa_verify(&params, &mut results);

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
        ziskos_fcall_param!(pk, 8);
        ziskos_fcall_param!(z, 4);
        ziskos_fcall_param!(r, 4);
        ziskos_fcall_param!(s, 4);
        ziskos_fcall!(FCALL_SECP256R1_ECDSA_VERIFY_ID);
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
            ]
        }
        #[cfg(feature = "inputcpy")]
        {
            use core::mem::MaybeUninit;
            let mut res: MaybeUninit<[u64; 8]> = MaybeUninit::uninit();
            ziskos_inputcpy!(res, 64);
            unsafe { res.assume_init() }
        }
    }
}
