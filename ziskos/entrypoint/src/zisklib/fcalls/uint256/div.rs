use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_param, zisklib::FCALL_UINT256_DIV_ID};
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::uint256::uint256_div;
    }
}

/// Given 256-bit unsigned integers `a` and `b`, it computes `(quotient, remainder)`
/// such that `a = b * quotient + remainder` with `0 <= remainder < b`.
///
/// Requires `b != 0`.
///
/// ### Safety
///
/// The caller must ensure that the input pointers are valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_uint256_div(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], [u64; 4]) {
    #[cfg(not(zisk_guest))]
    {
        let (quotient, remainder) = uint256_div(a, b);
        #[cfg(feature = "hints")]
        {
            hints.push(8);
            hints.extend_from_slice(&quotient);
            hints.extend_from_slice(&remainder);
        }

        (quotient, remainder)
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(a, 4);
        ziskos_fcall_param!(b, 4);
        ziskos_fcall!(FCALL_UINT256_DIV_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            (
                [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()],
                [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()],
            )
        }
        #[cfg(feature = "inputcpy")]
        {
            use core::mem::MaybeUninit;
            // TODO: generate an [u64;8] and after return 2 slides
            let mut quotient: MaybeUninit<[u64; 4]> = MaybeUninit::uninit();
            ziskos_inputcpy!(quotient, 32);

            let mut remainder: MaybeUninit<[u64; 4]> = MaybeUninit::uninit();
            ziskos_inputcpy!(remainder, 32);
            (unsafe { quotient.assume_init() }, unsafe { remainder.assume_init() })
        }
    }
}
