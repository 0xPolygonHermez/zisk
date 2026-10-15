use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_param,
            zisklib::FCALL_ED25519_FP_SQRT_RATIO_ID,
        };
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::ed25519::ed25519_fp_sqrt_ratio;
    }
}

/// Hint the square root of `u / v` in the base field of the Ed25519 curve (p = 2^255 - 19).
///
/// `fcall_ed25519_fp_sqrt_ratio` takes two 256-bit field elements as arrays of four `u64` values
/// and returns five `u64` values: the first is 1 if `u / v` is a square and 0 otherwise, the
/// remaining four are a canonical, even `r` with `v * r^2 = u` (square) or `v * r^2 = 2 * u`
/// (non-square; 2 is a non-residue mod p).
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_ed25519_fp_sqrt_ratio(
    u: &[u64; 4],
    v: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 5] {
    #[cfg(not(zisk_guest))]
    {
        let mut result: [u64; 5] = [0; 5];
        ed25519_fp_sqrt_ratio(u, v, &mut result);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(u, 4);
        ziskos_fcall_param!(v, 4);
        ziskos_fcall!(FCALL_ED25519_FP_SQRT_RATIO_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            [
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
            let mut res: MaybeUninit<[u64; 5]> = MaybeUninit::uninit();
            ziskos_inputcpy!(res, 40);
            unsafe { res.assume_init() }
        }
    }
}
