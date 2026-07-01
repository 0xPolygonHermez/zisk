use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_param, zisklib::FCALL_SECP256K1_GLV_DECOMPOSE_ID};
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    }
}

/// Hints the GLV decomposition of a scalar `k ∈ [0, n)` for the secp256k1 curve.
///
/// Returns `(k1, k2, sigma1, sigma2)` packed as 10 `u64` limbs:
///   `[k1[0..4], k2[0..4], sigma1, sigma2]`
/// where `k1, k2` are unsigned magnitudes (little-endian, each `< 2^128`) and `sigma_i ∈ {0,1}`
/// is the sign bit (`0` = positive, `1` = negative). They satisfy
/// `(-1)^sigma1 · k1 + (-1)^sigma2 · k2 · λ ≡ k (mod n)`.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify
/// the correctness of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_secp256k1_glv_decompose(
    k: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 10] {
    #[cfg(not(zisk_guest))]
    {
        use crate::zisklib::fcalls_impl;

        let mut params: [u64; 4] = [0u64; 4];
        params.copy_from_slice(k);

        let mut results = [0u64; 10];
        fcalls_impl::secp256k1::fcall_secp256k1_glv_decompose(&params, &mut results);

        #[cfg(feature = "hints")]
        {
            hints.push(results.len() as u64);
            hints.extend_from_slice(&results);
        }

        results
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(k, 4);
        ziskos_fcall!(FCALL_SECP256K1_GLV_DECOMPOSE_ID);
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
            ]
        }
        #[cfg(feature = "inputcpy")]
        {
            use core::mem::MaybeUninit;
            let mut res: MaybeUninit<[u64; 10]> = MaybeUninit::uninit();
            ziskos_inputcpy!(res, 80);
            unsafe { res.assume_init() }
        }
    }
}
