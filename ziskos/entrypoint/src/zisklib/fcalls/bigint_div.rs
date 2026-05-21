use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::FCALL_BIGINT_DIV_ID;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::bigint_div::bigint_div_into;
    }
}

/// Given unsigned big integers `a` (of `a.len()` limbs) and `b` (of `b.len()` limbs),
/// it computes `(quo, rem)` such that `a = b * quo + rem` with `0 <= rem < b`.
///
/// Requires `b != 0`.
///
/// Returns `(len_quo, len_rem)` — the number of limbs written to `quo` and `rem`.
///
/// ### Safety
///
/// The caller must ensure that the input pointers are valid and aligned to an 8-byte boundary.
/// The `quo` and `rem` slices must be large enough to hold the result.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bigint_div(
    a: &[u64],
    b: &[u64],
    quo: &mut [u64],
    rem: &mut [u64],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (usize, usize) {
    #[cfg(not(zisk_guest))]
    {
        let mut quo_vector: Vec<u64> = Vec::new();
        let mut rem_vector: Vec<u64> = Vec::new();
        bigint_div_into(a, b, &mut quo_vector, &mut rem_vector);
        quo[..quo_vector.len()].copy_from_slice(&quo_vector);
        rem[..rem_vector.len()].copy_from_slice(&rem_vector);
        let len_quo = quo_vector.len();
        let len_rem = rem_vector.len();
        #[cfg(feature = "hints")]
        {
            hints.push(len_quo as u64 + len_rem as u64 + 2);
            hints.push(len_quo as u64);
            hints.extend_from_slice(&quo_vector);
            hints.push(len_rem as u64);
            hints.extend_from_slice(&rem_vector);
        }

        (len_quo, len_rem)
    }
    #[cfg(zisk_guest)]
    {
        let len_a = a.len() as usize;
        ziskos_fcall_param!(len_a, 1);
        for i in 0..len_a {
            ziskos_fcall_param!(a[i], 1);
        }

        let len_b = b.len() as usize;
        ziskos_fcall_param!(len_b, 1);
        for i in 0..len_b {
            ziskos_fcall_param!(b[i], 1);
        }

        ziskos_fcall!(FCALL_BIGINT_DIV_ID);

        #[cfg(not(feature = "inputcpy"))]
        {
            let len_quo = ziskos_fcall_get() as usize;
            for i in 0..len_quo {
                quo[i] = ziskos_fcall_get();
            }

            let len_rem = ziskos_fcall_get() as usize;
            for i in 0..len_rem {
                rem[i] = ziskos_fcall_get();
            }

            (len_quo, len_rem)
        }
        #[cfg(feature = "inputcpy")]
        {
            let len_quo = ziskos_fcall_get() as usize;
            ziskos_inputcpy!(quo, len_quo * 8);
            let len_rem = ziskos_fcall_get() as usize;
            ziskos_inputcpy!(rem, len_rem * 8);

            (len_quo, len_rem)
        }
    }
}
