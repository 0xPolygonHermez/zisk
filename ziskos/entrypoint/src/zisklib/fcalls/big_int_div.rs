use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::FCALL_BIG_INT_DIV_ID;
    } else {
        use crate::zisklib::fcalls_impl::big_int_div::big_int_div_into;
    }
}

/// Executes the division of an unsigned integer of length `l` by another unsigned integer of length `s`.
///
/// ### Safety
///
/// The caller must ensure that the input pointers are valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_division(
    a_value: &[u64],
    b_value: &[u64],
    quo: &mut [u64],
    rem: &mut [u64],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (usize, usize) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let mut quo_vector: Vec<u64> = Vec::new();
        let mut rem_vector: Vec<u64> = Vec::new();
        big_int_div_into(a_value, b_value, &mut quo_vector, &mut rem_vector);
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
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        let len_a = a_value.len() as usize;
        ziskos_fcall_param!(len_a, 1);
        for i in 0..len_a {
            ziskos_fcall_param!(a_value[i], 1);
        }

        let len_b = b_value.len() as usize;
        ziskos_fcall_param!(len_b, 1);
        for i in 0..len_b {
            ziskos_fcall_param!(b_value[i], 1);
        }

        ziskos_fcall!(FCALL_BIG_INT_DIV_ID);

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
}
