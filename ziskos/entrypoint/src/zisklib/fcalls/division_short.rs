//! fcall_division_short free call
use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use crate::FCALL_DIVISION_SHORT_ID;
    }
}

/// Executes the division of an unsigned integer of length `l` by a u64.
///
/// ### Safety
///
/// The caller must ensure that the input pointers are valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_division_short(a_value: &[u64], b_value: &[u64; 4]) -> (Vec<u64>, [u64; 4]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        let len_a = a_value.len();
        ziskos_fcall_param!(len_a, 1);
        for i in 0..len_a {
            ziskos_fcall_param!(a_value[i], 1);
        }
        ziskos_fcall_param!(b_value, 4);

        ziskos_fcall!(FCALL_DIVISION_SHORT_ID);

        let len_quo = ziskos_fcall_get() as usize;
        let mut quotient = vec![0u64; len_quo];
        for i in 0..len_quo {
            quotient[i] = ziskos_fcall_get();
        }

        let mut remainder = [0u64; 4];
        for i in 0..4 {
            remainder[i] = ziskos_fcall_get();
        }

        (quotient, remainder)
    }
}
