#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::alloc_extern::vec;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use crate::alloc_extern::vec::Vec;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::FCALL_BIN_DECOMP_ID;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::bin_decomp::bin_decomp;
    }
}

/// Given an unsigned big integer `x`, it computes the binary decomposition of `x`,
/// returning the individual bits as a vector of `u64` values (each 0 or 1),
/// from most significant to least significant.
///
/// Returns `(len_bits, bits)` where `len_bits` is the number of bits and `bits[i]` is the `i`-th bit.
///
/// ### Safety
///
/// The caller must ensure that the input pointer is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bin_decomp(
    a: &[u64],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (usize, Vec<u64>) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let len_a = a.len();
        let bits = bin_decomp(a, len_a);
        let len_bits = bits.len();
        let bits_u64: Vec<u64> = bits.into_iter().map(|b| b as u64).collect();
        #[cfg(feature = "hints")]
        {
            hints.push(len_bits as u64 + 1);
            hints.push(len_bits as u64);
            hints.extend_from_slice(&bits_u64);
        }

        (len_bits, bits_u64)
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        let len_a = a.len() as usize;
        ziskos_fcall_param!(len_a, 1);
        for i in 0..len_a {
            ziskos_fcall_param!(a[i], 1);
        }

        ziskos_fcall!(FCALL_BIN_DECOMP_ID);

        let len_bits = ziskos_fcall_get() as usize;
        #[cfg(not(feature = "inputcpy"))]
        {
            let mut bits = vec![0u64; len_bits];
            for i in 0..len_bits {
                bits[i] = ziskos_fcall_get();
            }

            (len_bits, bits)
        }
        #[cfg(feature = "inputcpy")]
        {
            let mut bits: Vec<u64> = Vec::with_capacity(len_bits);
            ziskos_inputcpy!(bits, len_bits * 8);
            unsafe {
                bits.set_len(len_bits);
            }
            (len_bits, bits)
        }
    }
}
