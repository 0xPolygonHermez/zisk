use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_param, zisklib::FCALL_BN254_FP2_INV_ID};
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::bn254::bn254_fp2_inv;
    }
}

/// Compute the multiplicative inverse of a field element in the complex extension field of the BN254 curve.
///
/// `fcall_bn254_fp2_inv` operates on a 512-bit field element represented as an array of eight `u64` values,
/// and returns the inverse as an array of eight `u64` values.
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
pub fn fcall_bn254_fp2_inv(
    x: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let result: [u64; 8] = bn254_fp2_inv(x);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(x, 8);
        ziskos_fcall!(FCALL_BN254_FP2_INV_ID);
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
            let mut result: MaybeUninit<[u64; 8]> = MaybeUninit::uninit();
            ziskos_inputcpy!(result, 8 * 8);
            unsafe { result.assume_init() }
        }
    }
}
