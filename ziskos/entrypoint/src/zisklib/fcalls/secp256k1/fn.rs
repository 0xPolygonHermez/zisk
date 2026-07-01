use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_param, zisklib::FCALL_SECP256K1_FN_INV_ID};
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use lib_c::secp256k1_fn_inv_c;
    }
}

/// Compute the multiplicative inverse of a field element in the scalar field of the Secp256k1 curve.
///
/// `fcall_secp256k1_fn_inv` operates on a 256-bit field element represented as an array of four `u64` values,
/// and returns the inverse as an array of four `u64` values.
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
pub fn fcall_secp256k1_fn_inv(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    #[cfg(not(zisk_guest))]
    {
        let mut result: [u64; 4] = [0; 4];
        secp256k1_fn_inv_c(x, &mut result);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(x, 4);
        ziskos_fcall!(FCALL_SECP256K1_FN_INV_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()]
        }
        #[cfg(feature = "inputcpy")]
        {
            let mut res: core::mem::MaybeUninit<[u64; 4]> = core::mem::MaybeUninit::uninit();
            ziskos_inputcpy!(res, 32);
            unsafe { res.assume_init() }
        }
    }
}
