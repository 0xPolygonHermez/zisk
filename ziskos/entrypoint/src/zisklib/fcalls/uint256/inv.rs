use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param, zisklib::FCALL_UINT256_INV_ID};
    } else {
        use crate::zisklib::fcalls_impl::uint256::uint256_inv;
    }
}

/// Given an odd 256-bit unsigned integer `a`, it computes `a^(-1) (mod 2^256)`.
/// Returns `None` if no inverse exists.
///
/// ### Safety
///
/// The caller must ensure that the input pointer is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_uint256_inv(
    a: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    #[cfg(not(zisk_guest))]
    {
        let inv = uint256_inv(a);
        #[cfg(feature = "hints")]
        {
            if let Some(ref inv) = inv {
                hints.push(4);
                hints.extend_from_slice(inv);
            } else {
                hints.push(1);
                hints.push(0);
            }
        }
        inv
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(a, 4);
        ziskos_fcall!(FCALL_UINT256_INV_ID);

        let has_inv = ziskos_fcall_get();
        if has_inv == 0 {
            None
        } else {
            Some([ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()])
        }
    }
}
