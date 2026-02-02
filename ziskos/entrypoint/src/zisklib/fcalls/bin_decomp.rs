use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::FCALL_BIN_DECOMP_ID;
    } else {
        use crate::zisklib::fcalls_impl::bin_decomp::bin_decomp;
    }
}

/// Computes the binary decomposition of a NON-ZERO unsigned integer `x` into its bits.
#[allow(unused_variables)]
pub fn fcall_bin_decomp(
    x_val: &[u64],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (usize, Vec<u64>) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let len_x = x_val.len();
        let bits = bin_decomp(x_val, len_x);
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
        let len_x = x_val.len() as usize;
        ziskos_fcall_param!(len_x, 1);
        for i in 0..len_x {
            ziskos_fcall_param!(x_val[i], 1);
        }

        ziskos_fcall!(FCALL_BIN_DECOMP_ID);

        let len_bits = ziskos_fcall_get() as usize;
        let mut bits = vec![0u64; len_bits];
        for i in 0..len_bits {
            bits[i] = ziskos_fcall_get();
        }

        (len_bits, bits)
    }
}
