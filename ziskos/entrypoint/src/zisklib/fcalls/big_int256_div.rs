use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::FCALL_BIG_INT256_DIV_ID;
    } else {
        use crate::zisklib::fcalls_impl::big_int256_div::big_int256_div;
    }
}

/// Executes the inverse computation
///
/// `fcall_bigint256_div` performs an inversion of a 256-bit field element,
/// represented as an array of four `u64` values.
///
/// - `fcall_bigint256_div` performs the inversion and **returns the result directly**.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bigint256_div(
    a_value: &[u64; 4],
    b_value: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], [u64; 4]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let (quotient, remainder) = big_int256_div(a_value, b_value);
        #[cfg(feature = "hints")]
        {
            hints.push(8);
            hints.extend_from_slice(&quotient);
            hints.extend_from_slice(&remainder);
        }

        (quotient, remainder)
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(a_value, 4);
        ziskos_fcall_param!(b_value, 4);
        ziskos_fcall!(FCALL_BIG_INT256_DIV_ID);
        (
            [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()],
            [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()],
        )
    }
}
